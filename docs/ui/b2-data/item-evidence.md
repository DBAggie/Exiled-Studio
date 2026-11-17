# B-2 item evidence: in-game copies

The maintainer copied 27 items in the game on 2026-11-15 (PoE1, the build of the 26530219 bundle), with Ctrl+Alt+C, and took the plain and Alt (advanced) tooltip screenshots. Two items were copied at two quality levels, so there are 29 copies. The plan is `dev/active/ux-improvements/b2-item-evidence-plan.md`.

## What is in the repository

| Path | What |
|---|---|
| `game-copies/<item>.txt` | Each copy as the game wrote it. Lines from the chat around the copies were removed. There are no notes, account names, or character names. |
| `game-copies/pob/<item>.txt` | Our conversion of each copy into PoB's item text (`loom_serialization::legacy::game_copy::poe1`). Tests check it; rewrite it with `LOOM_BLESS=1` after a reviewed change. |
| `game-copies/pob-oracle/<item>.txt` | PoB v2.67.2's own reading of each copy: `ItemClass:ParseRaw`, then `BuildRaw`, by `tools/oracle/data/pob_items.lua` on the pinned oracle. |

**Flavour text.** The maintainer decided on 2026-11-15: "We can copy the flavour text." So the copies keep their flavour sections.

**Screenshots.** They carry GGG's art, so they stay out of the repository (ADR-000). Every number a test takes from them is written below.

The gem copy (Shield Crush) is evidence for the converter's refusal only: a gem is not an item to add. Its tooltip confirms the socket colour quality rule of P3-12i. Its quality was 20% with 10% from the socket colour, so the quality effect was 15%.

## Values: how the game computes them

A defence or weapon damage is `round((base + local flat) x (1 + local increased) x (1 + quality))`, rounded to the nearest whole number. This is what PoB and our engine compute (`loom_calc::poe1::phases::piece`; weapons in `poe1::hit`), so there is no divergence. The base rolls below come from the Alt view, which prints a base's value with its range.

| Item | Base roll (range) | Local lines | Quality | Shown |
|---|---|---|---|---|
| Grim Mangler, Totemic Maul | physical 57-85 | +27 to 45 physical, 89% increased | 0% / 9% | 159-246 / 173-268 |
| Brimstone Salvation, Gladiator Helmet | armour 178 | 43% increased | 0% / 16% | 255 / 295 |
| Storm Keep, Golden Plate | armour 616 | +29 | 0% / 9% | 645 / 703 |
| Ghoul Pelt, Royal Plate | armour 1360 | +306, 41% + 100% increased | 20% | 4818 (4818.07) |
| Soul Ascension, Carnal Mitts | evasion 90, energy shield 19 | 147% increased | 17% | 260, 55 |
| Foulborn Emperor's Vigilance, Steel Kite Shield | armour 165, energy shield 35 | 1120% increased | 40% | 2818, 598 (597.8) |
| Tempest Refuge, Ezomyte Tower Shield | armour 456 (412-474) | +257, 40% increased | 20% | 1198 |
| Loath Crown, Archdemon Crown | armour 207, energy shield 42 | 26% increased | 20% | 313, 64 (63.504) |

Other checks:
- **Block.** Tempest Refuge has 24% base block and 46% increased: 24 x 1.46 = 35.04, shown as 35. This does not tell `floor` from `round`, so the engine keeps PoB's `floor`.
- **Crit.** Widowmaker's local "increased Critical Strike Chance" applies to the weapon; its implicit "Global Critical Strike Chance" does not.
- **Weapon bases.** These match the bundle's bases:
  - Double Axe: 36-60 at 1.25 attacks per second.
  - Bastard Sword: 17-29 at 1.45.
  - Long Staff: 24-41 at 1.30, with 7.5% crit.
  - Iron Staff (warstaff): 14-42 at 7.60% crit.
  - Footman Sword: 39-65.

## The copy format

The converter's rules are in its module docs. Each follows PoB's `ParseRaw`, which reads the same copies when they are pasted into PoB. What the copies showed:

- **Sections** are separated by `--------`.
  - Header: `Item Class:`, `Rarity:`, the name, and for rare and unique items the base. Magic and normal items carry the base in the name.
- **Properties:**
  - Values carry `(augmented)`.
  - Weapon values (damage, crit, attacks per second, range) and block are shown values; the calculation computes them.
  - Defences (`Armour: 1198`) are kept as written. The engine derives the base roll from them, as PoB does (below).
- **Requirements** include socketed gems' requirements, without the tooltip's `(gem)` mark. An unmet requirement is marked `(unmet)`.
  - PoB takes the copy's level only for an item without sockets, and the base's otherwise. The converter does the same.
- **Modifier headers:**
  - The format is `{ <Kind> Modifier "<affix>" (Tier: n | Rank: n) — <Tags>[  — 20% Increased] }`.
  - Kinds seen: Prefix, Suffix, Implicit, Corruption Implicit, Searing Exarch Implicit (Grand), Eater of Worlds Implicit (Greater), Unique, Foulborn Unique, Master Crafted Prefix and Suffix, Fractured Prefix and Suffix.
- **Values** are written `v(min-max)`, and a value may lie outside its range (a legacy roll, socket colour quality). `— Unscalable Value` is written with an em dash.
- **Reminder lines** are written `( ... )`; a modifier can have several. `Akoya(...)` on the timeless jewel is part of its line.
- **Enchants** are marked `(enchant)` on every line, the reminder lines included, and have no header.
- **Flavour text** is a section of its own. Jewels, flasks, and gems add a usage hint after it (`Place into ...`, `Right click to ...`).
- **No fixed order** for `Corrupted`, `<Influence> Item`, `Fractured Item`, and the flavour text.
- **The tooltip's two views:**
  - The plain view sums the same stat over modifiers (41% + 100% shows as "141% increased Armour") and sorts lines in the game's stat order, fractured lines first.
  - The Alt view lists each modifier with its header, in modifier order. The item card follows the Alt view.

## PoB's reading against ours

`crates/loom-serialization/tests/game_copy_fixtures.rs` reads both texts with the PoB item reader. It compares name, rarity, base, specs, flags, and each modifier's section and shown value. All 28 item copies agree except the four lines below, which the test lists exactly:

| Copy | PoB's reading | Ours | Which is right |
|---|---|---|---|
| Combatant's Iron Staff of Absorption | the property word "Warstaff" as a modifier | dropped | The game shows a property. |
| Widowmaker | "Rune Dagger" as a modifier | dropped | The game shows a property. |
| Continuous Divine Life Flask of Quashing | "Recovers 2400 Life over 3.50 Seconds" as a modifier | dropped | It is the flask's property. |
| Replica Dreamfeather | "Melee Hits have 13% chance to Fortify" | 15% | The game rolled 15. PoB cannot parse the line, and `ParseRaw` then sets its roll to the default 0.5 (`Item.lua:1321`). |

Other differences:
- **Quality on paste.** PoB's paste also runs `NormaliseQuality`, which raises quality to 20% on items that are not unique or corrupted. We keep the copy's quality: the game's.
- **Line order.** PoB orders explicit lines by stat order when it writes them back. We keep the copy's order.
- **Unknown affix names.** PoB marks a crafted line whose affix name it cannot find (`"Upgraded"`) `{custom}`. We keep it as a crafted line; the value is the same.

## Adding a pasted item (E3)

On the Equipment page, **Paste an item** takes a game copy or PoB's item text. So does a paste anywhere on the page outside a text field.
- **The command.** App command `add-item { text, slot? }` (app API v1, additive) reads the item and resolves it as an imported one. It is one undoable step (history kind `item-added`), kept through the recovery history; refusals are `APP_ADD_ITEM`.
- **The slot.** With a slot, the item goes into that slot of the active item set, or of a new set when the build has none. The slot must fit the base's item class. A two-handed weapon empties the off hand, as in the game; a bow keeps a quiver.
- **The text** is capped at 64 KiB and logged only by its size and SHA-256.
- **Tests.**
  - The workflow test `a_pasted_game_copy_should_become_an_item_and_undo_should_remove_it` covers add, equip, undo, and the refusals: a slot that does not fit, a gem, an unknown base, and a text over the cap. It goes through the recovery history as every edit does; no test of its own covers that.
  - On the real bundle, every copy adds and the slot rules hold (`game_truth_items.rs`).
  - In the UI, `paste-item.test.tsx`.
- **Not here:** an MCP tool for it.

## The base roll from a shown defence (E4)

An item that shows a defence (`Armour: 1198`) but has no `ArmourBasePercentile` line now gets the percentile that gives the shown value back. It used to get a perfect roll and a `base-percentile` reason. The formula is PoB's (`Item.lua:2323-2337`): `round(clamp((shown / ((1 + inc) x (1 + quality)) - flat) / range, 0, 1), 4)`.

One deliberate difference: PoB subtracts only the defence's own flat there, and adds hybrid local flats ("+# to Armour and Evasion Rating") afterwards. So on a hybrid piece with such a line, PoB's value is not the one the item shows. Ours subtracts every local flat and gives the shown value back. PoB writes the percentile into its own items, so builds exported from PoB never reach this path; only pasted copies do.
- **Calc evidence** (2026-11-15, this workstation):
  - **Run.** `calc-run` of the public set (136 builds, the `attack-hits.md` command, scope `calc-scope-poe1.json`) on the flavour bundle `bundle-poe1-tree-a`, made with the CLI at `dev` (`c0623d3`) and at this branch (`0bde2a0` plus the review's test and comment). Two runs at the branch give byte-identical documents.
  - **Result.** In every one of the 135 documents, each field's value, state, and reasons are equal; the summary counts are equal too (2,270 complete, 176 partial, 482 unsupported, 312 indeterminate). 38 documents differ only in the graph's node numbering.
  - **Not rerun.** No `calc-parity` rerun was needed: every compared value is the same.
  - **Corpus** (main workstation, with the data run below): `calc-run --anonymous` of the 21 corpus builds on the new app bundle, with the CLI at `dev` (`c0623d3`) and at this branch (`ce00a46`). In all 21 documents the result's state, reasons, and fields (values, states, reasons, node references) are equal; only the `graph` section differs. Summary counts equal (141 unsupported, 363 indeterminate).
- **Register.** No fixture reaches the difference, so there is no `oracle-compare` difference for a register entry to cover; the register is kept by fixture and field (`docs/parity/README.md`). If a fixture ever reaches it, add a `game-truth` entry with this evidence: the game's tooltip is the shown value.
- **Pinned.** `item_values_should_be_the_calculations_own_nodes` (`crates/loom-calc/tests/poe1.rs`) covers the hybrid case: an evasion base of 10 to 20, `local_evasion_rating_and_energy_shield` +10, and a shown 25 give 25 back. PoB's way gives 30.
- **A shown value of 0 or less** gives no percentile, as in PoB (`armourData.Armour > 0`), so the roll is the top. A copy of a base with a defence range never shows 0.

## Card values (E5)

The item card shows the item's own values as the game's tooltip prints them, from the calculation, not from the written lines:
- a weapon's damage per type, critical strike chance (two places), and attacks per second;
- a shield's chance to block;
- the defences, which take the place of the written `Armour:` lines.

How they are computed:
- **Entry.** `loom_calc::item_values` collects a build and builds the same nodes the phases build: `phases::piece`, `phases::shield_block` (split out of the block phase, same graph), and `hit::weapon_data`. It has no formula of its own.
- **The build.** The app (`loom_app::card_values`) calculates the item alone: a new level 1 build with the item in a slot that fits its class (a weapon in the main hand, a shield in the off hand, an armour piece in its slot) and the item's range ends. Only local lines act on these values.
- **Partial.** A card is marked partial when a line of the item is not resolved, or when its base is not modelled or data is missing; any of these could hide a local line. An unmapped stat does not mark it: every stat that acts on these values is mapped, so an unmapped one acts on something else.
- **No ward.** Ward is not shown.

Game truth (`each_cards_values_should_equal_what_its_copy_prints`, real bundle): the card's values printed as the game prints them, against each copy's property lines.
- 18 copies have values, and all 18 print exactly the copy's lines. That covers the weapons' damage per type (the bow's elemental ranges included), crit and attack rate, the shields' block, and the defences.
- **Replica Dreamfeather** (fixed 2026-11-17, "Unique lines by their base" below) printed 1.50 attacks per second for the game's 1.65. Its "(5-10)% increased Attack Speed" was one of three lines the resolver found ambiguous (local or global), so the card left it out and was marked partial. It now prints 1.65.
- Loath Crown is marked partial (lines without a template), and its values still match.

## Requirements (E6)

The game's `Requirements:` include the socketed gems' requirements, so a socketed item's copy can't give the item's own.

What the card shows:
- **The copy's level.** Kept only for an item without sockets, as PoB's paste does (`Item.lua:1361-1367`, written as `LevelReq`).
- **Otherwise the base's.**
  - The base's level now follows PoB's exporter (`bases.lua:277-293`, now `loom_data::bundle::required_level`, shared with data-parity's `base_requirements`): the drop level for a weapon or armour (above 4) or a flask (above 2), raised to `floor(0.8 x level)` of its implicits.
  - It used to be the drop level of any base, so Lethal Pride showed level 20, where the game shows no requirement for a jewel.

Game truth (`each_cards_requirements_should_be_the_copys_without_its_gems`, real bundle):
- The 6 copies without sockets that print a level show the copy's requirements exactly.
- The 4 jewels show none.
- Every socketed card's requirements are at most its copy's.

## Affix headers (E8)

The copies' headers are the game's own Alt view. Each prefix, suffix, and named implicit header was compared with the card's affixes on the real bundle (`each_copys_affix_headers_should_equal_the_cards_affixes`), comparing kind, affix name, tier, crafted, and tags.
- The card names 45 of the 61 headed affixes. It leaves the others out: their lines have more than one reading in the data, such as two mods summed into one line, or stats on two lines.
- **Ghoul Pelt.** The card named the suffix "of Stone Skin", which the item does not have. The item's two "% increased Armour" lines carry one stat, so the inference set them aside. The third line, "16% increased Stun and Block Recovery", then looked like a single-stat suffix. In the game it is the second line of the hybrid prefix "Mammoth's". Fixed in `loom_parser::import::affixes`: a line that a fitting row could join to a set-aside line is set aside too. Test: `a_line_a_set_aside_stat_could_join_should_get_no_header`.
- **Influence tiers** (fixed 2026-11-17). Tempest Refuge's "of Shaping" had no tier on the card; the game shows Tier 1. The card gave influence mods no tier, while the game counts tiers among the influence's own rows. `affixes::tier` now counts, for an influence row, the higher-level rows of the same mod type and influence on each influence tag the row spawns on (`shield_shaper`), and gives a tier only when every such tag agrees. "of Shaping" (`RecoverLifePercentOnBlockUber1_`, level 68) is the only row of its type on `shield_shaper`: Tier 1. No difference is left; test `an_influence_mod_should_get_its_tier_among_its_influences_rows`.

## Socket caps (E9)

The copies agree with the item type files' caps (`card-data.md`, "Maximum sockets").
- **Seen.** Two-handers and body armour at item levels 14, 25, and 46 have at most 3, 4, and 5 sockets.
- **Built.** The bundle builder reads each base's `socket_info` from the nearest file of its item type chain (`item_tags::ItemTags::socket_info`). It keeps the counts that roll (weight above 0, item level below the game's 9999 "never") and writes `poe1/socket_caps.json`, an optional file with no schema bump.
- **Shown.**
  - The card gets `max_sockets`, the most the base rolls at the item's level; at the highest level when the item has no level written.
  - The Items card states it.
  - Both views note an item with more sockets than that. This is display only, never a calculation state.
- **Tested.**
  - The fixture bundle carries made-up caps in the game's form.
  - Tests cover the validator (duplicate, unknown base, empty, out of order), the item levels 1, 14, 25, 46, 72, and the card.
- **Data run** (main workstation, 2026-11-15; "Main workstation data run" below): the real file has caps for 863 bases, following the game's thresholds (body armour and two-handers: 4 sockets from item level 25, 5 from 35, 6 from 50; helmets stop at 4, one-handers and shields at 3).
- **Corpus:** of 1,055 items in the 21 builds, 375 get a cap and **none has more sockets than its cap**. 20 socketed items get no cap: Stygian Vise and Unset Ring bases (with Darkness Enthroned), whose sockets come with the base, so their item type files roll no count. Most items in the corpus have no item level written, so they are checked at the highest level.
- **Difference from PoB.** PoB's per-base `socketLimit` has no item-level thresholds, and it gives Gnarled Branch and Corroded Blade 6 where the game data gives 3. The card follows the game; this is display only.

## Flavour text

The card reads unique flavour text from the bundle (#116). Each unique copy's flavour section was checked against it on the real bundle (`crates/loom-app/tests/game_truth_items.rs`):
- All 9 uniques with flavour text agree, after the data run's fix below.
- **The Light of Meaning** did not, on #116's bundle. The game shows "To name a thing is to give it power.", but PoB v2.67.2 pairs the unique with `UniqueJewel181` ("Faith given under false pretenses still carries the same power."), and #116's bundle took PoB's pair.
  - **Why.** The Light of Meaning shares its art with That Which Was Taken, whose text is `UniqueJewel181`; both PoB's exporter and #116's art join give each stash row its art's text. The game's line is `FlavourText` row `UniqueJewel183_`, which no art reaches.
  - **Fix.** `("UniqueJewel183_", "The Light of Meaning")` in `poe1::FLAVOUR_PAIRS` (75 pairs); the flavour test now expects no difference. That Which Was Taken keeps `UniqueJewel181` (no copy of it yet).
  - **PoB.** The bundle no longer has PoB's `The Light of Meaning|UniqueJewel181` row: `data-parity` rule `flavour-game-copy-over-shared-art` (`pob-derivation`, count 1), with the copy as its evidence (`docs/phase-3/data-parity.md`, "B-2: table `flavour_text`"). Not a divergence-register entry: the register covers calc output fields by fixture, and `data-parity`'s rules have no `game-truth` category; a `pob-derivation` rule is how a data difference where PoB's exporter derives the value is recorded.

## Tree flavour (E10)

The tree screenshots show a keystone's flavour text under its lines, and an ascendancy's quote. The maintainer chose the game files over GGG's tree export as the source (2026-11-15).
- **Built.** The bundle builder (`tools/parity-cli/src/tree_flavour.rs`) reads two tables:
  - `PassiveSkills.FlavourText` by `PassiveSkillGraphId`;
  - `Ascendancy.FlavourText` and `RGBFlavourTextColour` (`r,g,b`) by `Id`.
  It writes rows with text only, into `poe1/tree_flavour.json`. This is an optional file with no schema bump; the validator checks for duplicates and empty texts.
- **Shown.**
  - The tree layout carries the texts: `TreeNodeView.flavour`, `TreeAscendancyView.flavour` and `flavour-colour`.
  - The node hover card shows a node's flavour text, or the ascendancy's quote on an ascendancy start.
  - The list view shows a node's text.
  - The quote is under the ascendancy picker.
  - The text is set as muted italic. The game's colour is carried but not drawn, because each colour pair in the UI is contrast-checked (`pairs.ts`).
- **Tested.**
  - The fixture bundle has made-up texts.
  - Tests cover the validator, the reader's rows, the layout (`the_layout_should_carry_the_trees_flavour_texts`), and the screen (`tree.test.tsx`).
- **Data run** (main workstation, 2026-11-15):
  - **Fix.** The game's colour column is `RGBFlavourTextColour`; the reader asked for `FlavourTextColour`, which the extract does not have, so every quote would have shipped without its colour. Fixed, with the reader's test.
  - **Real file.** 119 passive texts and 18 ascendancy quotes, each with its colour.
  - **Against GGG's export at 3.29.1:** all 57 keystone texts of the export's tree are in the file, each equal to the export's (line by line, padding trimmed); the other 62 rows are passives not on the 3.29.1 tree, never shown. All 18 quotes and their colours equal the export's (`175,90,50` is `af5a32`).
  - **Left without a text:** Scion's Ascendant, Reliquarian, and Luminary. The game data and the export both have no quote for them (the export has 21 ascendancies, 18 with a quote; the plan's "21 with a quote" was wrong). No keystone of the tree is left without one.

## Attribute totals on the tree (E11)

The game's passive tree shows the character's Strength, Dexterity, and Intelligence next to the points. The tree screen now does too: it shows the totals from the open build's last calculation, so they follow every allocation. The line is hidden while a total is not computed. The Inventory panel's requirement check reads the totals the same way (`util.ts`, `useAttributes`). Test: `tree.test.tsx`.

## Main workstation data run

Done on this PR's branch on 2026-11-15 (the maintainer asked for it before the merge), Steam build 26530219. `local/b2ev/evidence.sh`, from clean commit `ce00a46` (the two fixes above):
- **Input.** #116's extract (`local/b2card/f/extract-a`): the extractor is unchanged since it was made (`git diff d2f5f54 ce00a46 -- tools/extractor` is empty). `PassiveSkills`, `Ascendancy`, `FlavourText`, and the item type files are in it.
- **`bundle`** twice, byte-identical; `bundle-check` no findings; `bundle.json` SHA-256 `2e9a11027fd1f0666b939432b97cb46c3b0cbafeb1e17d15146c0303f891f80a`. Against #116's bundle: `poe1/socket_caps.json` (863 bases) and `poe1/tree_flavour.json` are new, `poe1/unique_flavour.json` differs only in The Light of Meaning's row, and `bundle.json` differs.
- **`data-parity`** at PoB export format 8, twice byte-identical: 614 differences (not-carried 163, patch-delta 130, pob-derivation 241, representation 80), `uncategorized` 0, 0 failed rules, 371 overlay rows; summary `data-parity-poe1-26530219-evidence.json`. The one new difference is The Light of Meaning's (above).
- **`tree-layout`**, then `bundle-check` clean; against #116's tree bundle, the same files differ.
- **`calc-run`** (public 136, corpus 21) on #116's app bundle and twice on the new one: every document identical. **No calculation changed.** The code's before and after on the corpus is under E4.
- **Real-bundle tests** (`game_truth_items` 7, `game_truth_affixes` 3): all pass. The flavour test checks 9 uniques with no difference; the other expected lists are unchanged (Replica Dreamfeather's attack rate, Tempest Refuge's tier).
- **Socket caps and tree flavour:** E9 and E10 above.

The app bundle is in OneDrive `Exiled-Studio-bundles` as `bundle-poe1-26530219-schema4-evidence.zip` (8.2 MB, top folder `bundle-poe1-tree-a/`), SHA-256 `ffa2dd01f2832ffa72eb4fa953e3ea60067a1b749b680d373203099d857dc973`, with its `.sha256` file (checked there; `bundle-check` clean on the unzipped copy). It replaces `bundle-poe1-26530219-schema4-flavour.zip`. Not committed (GGG's sprite sheets).

**On the other workstation** (2026-11-15): the zip's hash checked, unzipped to `local/b2ev/bundle-poe1-tree-a`, `bundle-check` clean, and the ignored real-bundle tests pass on it: `game_truth_items` (7), `game_truth_affixes` (3), and the real-bundle tests of `tree_workflow` and `loom-tree`. The files hold 119 passive texts, 18 quotes with their colours, and caps for 863 bases, as above.

## Lines that stay reasons

On the real bundle every copy adds, and every line resolves or warns visibly. Warnings left:
- "Socketed Gems are Supported by ...";
- the timeless jewel's lines;
- the flask's Taunt;
- "+% to Item Rarity".

Replica Dreamfeather's three lines that were ambiguous (`LINE_AMBIGUOUS`) resolve since 2026-11-17 (below).

## Unique lines by their base (2026-11-17)

A unique's line that reads as a local or a global stat was left ambiguous: the resolver's `local-stat` step (the base decides: a weapon keeps a weapon-local reading, armour an armour-local one, any other base the global one) ran on non-unique items only, since Spike B's gate counted those. It now runs on uniques too, as the game and PoB read them:
- **The game:** Replica Dreamfeather's "(5-10)% increased Attack Speed" is the sword's own; the copy prints 1.65 attacks per second (1.50 x 1.10).
- **PoB:** reads a weapon's or armour's local lines by the item's type, whatever its rarity.

**One guard.** A unique's readings are backed by unique rows of any item, a jewel's among them. With the step alone, Windscream's "+(10-15)% to all Elemental Resistances" on boots read as Thread of Hope's two jewel stats (both marked local, and boots carry the armour locality), and the build's resistances dropped. On a unique the step therefore decides only a plain pair, one stat each (note `local-stat-unique-not-a-pair` otherwise). Test: `local_stat_should_decide_a_uniques_plain_pair_only`.

**Evidence** (`local/card-fix/`, this workstation, the schema 4 evidence bundle `bundle-poe1-tree-a`): `calc-run` before and after the change, the public fixtures (136) and the private corpus (21), the new run twice.
- The two new runs are byte-identical.
- **Public:** 9 documents change. Four values move, each toward PoB's (`local/p3/close/evidence/oracle`); the other five documents change reasons only.

  | Build | Field | Before | After | PoB |
  |---|---|---|---|---|
  | OccVortex | energy shield | 1012 | 1236 | 1611 |
  | arakaalis-raise-spider-occultist | energy shield | 1742 | 1962 | 4688 |
  | ethereal-knives-mines-assassin | evasion | 2365 | 2514 | 2930 |
  | poisonous-concoction-of-bouncing-pathfinder | evasion | 7016 | 7699 | 25807 |

  Reason entries (a line's reason repeats on each field it may touch): 204 `ambiguous` entries gone, 68 added, from 4 lines that now resolve. Two are `unmapped-stat` `local_attack_speed_+%` on two dual-wielding builds' unique weapons (dual wielding is not modelled), two are `scaled-line` (catalyst, modifier magnitude). All four fields are `partial` or `indeterminate`, so `calc-parity` (against the close-out oracle run) is unchanged: 0 unexpected differences before and after, identical summaries.
- **Corpus:** 6 documents change, 6 values, each toward PoB's (`local/p3/p3-5/evidence-p4/oracle-corpus`): armour in four builds, evasion in two (one ends 2 above PoB's 198, from 25 below). 244 reason entries gone, none added.
- **Real-bundle tests:** `game_truth_items` 7 and `game_truth_affixes` 3 pass; Replica Dreamfeather's card prints the game's values and resolves every line, and no affix header differs from the game's.
