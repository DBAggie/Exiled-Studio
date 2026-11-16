# B-2 item card: game data for flavour text, mod tag names, and socket caps

Main workstation, 2026-11-15, PoE1 build 26530219 (the depot download in `local/b2/steam-26530219`). This answers the plan's Q-B2-1 and Q-B2-2 (`dev/active/ux-improvements/b2-item-data-plan.md`) and the card's maximum sockets (`docs/ui/ux-walkthrough.md`, "Still not done").

Inputs: the tree data run's extract (`local/tree-data/extract-26530219`, 101 tables, clean extractor `565f5b0`), and a research extract for this page only (`local/b2card/extract-research`, 105 tables, all Ok, dangling keys 0). The research extract added `Words`, `UniqueStashLayout`, and `KeywordPopupItemReference` as roots through an uncommitted patch to `tools/extractor/src/extract.rs` (reverted; its manifest says `dirty: yes`). Nothing from it is in a bundle.

## Summary

| Question | Answer | In the data? | Bundle change |
| --- | --- | --- | --- |
| Q-B2-2: mod tag names | `Tags.Name` (the fourth column) | Yes: 25 named tags, the rest empty | `data/tags.json` reads `Tags.Name`; PoE1's hand list `MOD_TAG_NAMES` removed |
| Maximum sockets | The `Sockets` component of the item type files: `socket_info` | Yes, by item level | None in this PR (the maintainer decides; see below) |
| Q-B2-1: unique flavour text | `FlavourText`, joined by name convention, no key | Partly: 1,302 of 1,337 unique names by the join; fated versions need a list | `poe1/unique_flavour.json` (maintainer 2026-11-15: "add flavour text"); the card shows it |

## Q-B2-2: mod tag names

The schema's `Tags` table has four columns: `Id`, `HASH32`, `DisplayString`, `Name`. B-2's data PR 1 read `DisplayString`, which holds monster labels only (20 rows: `pinnacle_boss` "Pinnacle Atlas Boss", `synthesised_monster` "Synthesised", ...). `Name` holds the mod tag names: 25 of the 1,389 rows have one.

```
ailment Ailment, armour Armour, attack Attack, attribute Attribute, aura Aura,
caster Caster, chaos Chaos, cold Cold, critical Critical, curse Curse,
damage Damage, defences Defences, drop Drop, elemental Elemental,
energy_shield Energy Shield, evasion Evasion, fire Fire, gem Gem, life Life,
lightning Lightning, mana Mana, minion Minion, physical Physical,
resistance Resistance, speed Speed
```

Every tag the game's copies hide has an empty `Name`: `resource`, `physical_damage`, `elemental_damage`, `caster_damage`, `chaos_damage`. Every tag the copies show has one: Urchin's `— Life, Defences, Armour` (tags `life`, `defences`, `resource`, `armour`), Thorny `— Damage, Physical` (`damage`, `physical_damage`, `physical`), `— Life, Defences, Evasion` and `— Life, Defences, Energy Shield`, and poedb's `Attribute`, `Chaos, Resistance` (the plan's Q-B2-2 research lists the copies). So the Alt view prints a mod's tags in the mod's order and skips a tag with no `Name`.

What changed against the hand list: it named `bleed`, `poison`, and `block`, which have no `Name` (the game hides them), and missed `drop`. `gem` and the other 23 agree.

PoE2's schema leaves `Tags`' fourth column unnamed (its own fifth is `UiHints`), so the bundle reads `Tags.Name` where the column exists and `DisplayString` otherwise (`tools/parity-cli/src/bundle.rs`, `tag_names`). No data schema change: `TagName` is still `{id, display}`. An app built before this PR reads the new bundle: its rule list is now empty, so it uses the tag table, as it already did when the list was empty. This PR's app reads the older bundles, which still carry the rule list. Those keep the old names (with `bleed`, `poison`, `block`) until the bundle is rebuilt. **PoE2 mod tag names are not solved:** its `DisplayString` holds labels, and whether its unnamed column holds the names, as in PoE1, is unchecked (no PoE2 Alt view to compare yet).

## Maximum sockets

No table holds socket caps: not `ItemClasses` (31 columns, none about sockets), not `BaseItemTypes`, and none of the schema's PoE1 tables with "Socket" in a name or column (`SocketNotches`, `SocketedItemArtOverrides`, `CraftingBenchOptions.Sockets`, ...). The caps are in the item type files' `Sockets` component, already extracted under `item_types/`. `socket_info` is a list of `count:minimum item level:weight`; 9999 means never. The base's `InheritsFrom` chain picks the nearest file that sets it:

| Item type file | `socket_info` | Maximum |
| --- | --- | --- |
| `Armours/AbstractArmour` (helmets, gloves, boots) | `1:1:100 2:1:90 3:2:80 4:25:30 5:9999:20 6:9999:5` | 4 from item level 25, else 3 (2 below 2) |
| `Armours/BodyArmours/AbstractBodyArmour` | `1:1:50 2:1:120 3:2:100 4:25:30 5:35:5 6:50:1` | 3, 4 from 25, 5 from 35, 6 from 50 |
| `Armours/Shields/AbstractShield` | `1:1:50 2:1:120 3:2:100 4:9999:30 5:9999:20 6:9999:5` | 3 |
| `Weapons/AbstractWeapon` (one-handers, bows by their own chain below) | `1:1:50 2:1:120 3:2:100 4:9999:30 ...` | 3 |
| `Weapons/TwoHandWeapons/AbstractTwoHandWeapon` | `1:1:50 2:1:120 3:2:100 4:25:30 5:35:5 6:50:1` | as body armour |
| `.../FishingRods/AbstractFishingRod` | `... 4:25:30 5:9999:20 6:9999:5` | 4 from 25 |
| `.../Staves/Staff1` (Gnarled Branch) | `... 4:9999:30 ...` | 3 |
| `.../TwoHandSwords/TwoHandSword1` (Corroded Blade) | `... 4:9999:30 ...` | 3 |
| `Rings/AbstractSocketableRing`, `Amulets/AbstractSocketableAmulet`, `Belts/AbstractSocketableBelt`, `Quivers/AbstractSocketableQuiver` | `0:1:1 1:9999:100 ...` | 0 rolled |

PoB v2.67.2 hard-codes a per-base `socketLimit` (`src/Data/Bases/*.lua`, from its export directive `socketLimit`) with no item-level thresholds: 6 for body armour and two-handers, 4 for helmets, gloves, boots, and fishing rods, 3 for one-handers and shields. Two differences from the game's data: PoB gives Gnarled Branch and Corroded Blade 6, the files give 3; and PoB ignores the item-level thresholds. Both are display only (the calculation does not read socket caps).

## Q-B2-1: unique flavour text

`FlavourText` (2,962 rows; `Id`, an unnamed `i32`, `Text`) holds it. No table has a key from a unique item to it: in the client the only references are `BaseItemTypes.FlavourTextKey` (1,306 bases: currency, cards, starter bases) and `KeywordPopupItemReference.FlavourText` (6 rows, all null). The path is a naming convention:

1. `UniqueStashLayout` (1,556 rows): `WordsKey` (the unique's name, `Words.Text2`) and `ItemVisualIdentityKey` (its art).
2. `ItemVisualIdentity.Id` equals a `FlavourText.Id`, sometimes after dropping trailing underscores (`UniqueJewel165__`).

Skipping maps, alternate art, Atlas upgrades, and heist quest items (174 rows, as PoB's export does): 1,215 rows match an Id exactly, 101 after stripping underscores, 66 have no row. That is 1,302 of 1,337 unique names with a text. Tabula Rasa has none, as in the game.

Three uniques end to end:

| Unique | `UniqueStashLayout` row | `Words` row (`Text2`) | `ItemVisualIdentity` row, Id | `FlavourText` row | Text |
| --- | --- | --- | --- | --- | --- |
| Kaom's Heart | 26 | 72 | 8379, `UniqueBodyStr1` | 31 | The warrior who / fears will fall. |
| Goldrim | 101 | 157 | 9433, `UniqueHelmetDex3` | 135 | No metal slips as easily through the fingers as gold. |
| Headhunter | 202 | 307 | 10319, `UniqueBelt7` | 285 | "A man's soul rules from a cavern of bone, learns and / judges through flesh-born windows. The heart is meat. / The head is where the Man is." / - Lavianga, Advisor to Kaom |

Where the convention fails:

- **Upgraded and fated versions share the art.** 63 art Ids with a text are used by 127 names: Kaom's Sign and Kaom's Way (`UniqueRing2`), Blackheart and Voidheart, Bramblejack and Wall of Brambles, ... The join gives both the base version's text. The upgraded version's stash row points at the base's with `BaseVersion` (181 rows), and its own text is a `FatedUniqueN` row (61 rows; Kaom's Way is `FatedUnique1`, "Kaom guided his Karui across the sea / and spilled their blood on foreign soil."), which nothing links.
- **One name, several texts.** Grand Spectrum (9 arts), Combat Focus (3), Lord of Steel (3), and Stormblood (2) have a text per variant, so the name alone does not pick one.
- **Text under another art Id.** Agnerod North, South, and West share `UniqueStaff8`'s text; the Descent and Demigod's uniques use other Ids.

PoB v2.67.2 does the same join (`src/Export/Scripts/flavourText.lua`) plus a hand list of 105 forced `(Id, name)` pairs for these cases, and ships `src/Data/FlavourText.lua` (1,356 entries, 1,315 names). Against it, the plain join has 32 names PoB lacks (PoB keys by art Id, so for a shared art only one name survives) and lacks 45 that PoB's hand list adds.

**Shipped** (maintainer 2026-11-15: "add flavour text"; socket caps are another session's work):

- **Extraction:** `UniqueStashLayout` and `Words` are PoE1 roots (`tools/extractor/src/extract.rs`, `FLAVOUR_TABLES_POE1`); `UniqueStashTypes` comes with them; `Words` was already extracted (`EssenceType` references it). 103 tables.
- **Converter** (`tools/parity-cli/src/flavour.rs`): the art join as above, skipping what PoB's export skips, and also skipping an upgraded version (`BaseVersion` set) whose art is its base's, so no unique gets another's text (56 rows). Then a PoE1 list, `FLAVOUR_PAIRS` in `tools/parity-cli/src/poe1.rs`: 74 `(FlavourText ID, name)` pairs, each replacing what the join gives that name. They are the pairs PoB v2.67.2 cites (`forcedNameList`) less the 31 the join already gives: 73 add a name the join cannot reach (fated uniques, Agnerod, the Descent and Demigod's uniques), and one corrects the join (Atziri's Reflection is `FatedUnique27`, not Atziri's Mirror's text). A pair whose ID the table lacks fails the bundle.
- **Bundle:** `poe1/unique_flavour.json`, `loom_data::bundle::poe1::UniqueFlavour` `{name, id, text}`, the game's text with its markup, `\r\n` made `\n`. Optional, so no data schema change; the validator checks keys (`name:id`) unique and present and text not empty. 1,332 rows for 1,319 names, 247 KB.
- **Card** (`loom_app::view`, app API v1 `ItemCard.flavour`, additive): a unique or relic whose name has exactly one text gets it as lines: `<<...>>` glyph runs (Harbinger script, no font) dropped, `<default>{...}` lines in the default colour (the Altar of Sacrifice notes), other markup removed, one gap where the game leaves blank lines. A Foulborn unique reads its plain version's text, as PoB's tooltip does. Grand Spectrum, Combat Focus, Lord of Steel, and Stormblood (a text per variant) show none until a variant can be told apart.
- **Tooltip:** the lines after the flags (`Corrupted`, ...), in the game's flavour brown, as PoB's tooltip orders them; the game's own order is to be checked on the maintainer's screenshot.
- **Gaps:** of the 56 upgraded versions with their base's art, the list gives 28 a `FatedUniqueN` text; 28 have no evidenced pair, so no text (Voidheart, Wall of Brambles, Cragfall, The Oak, ...). Of the 61 `FatedUniqueN` rows, 29 are paired: those 28 and Atziri's Reflection. A pair for each of the rest needs evidence (an in-game copy or screenshot). Checked on the data: no name has two rows with the same text, every `<default>` starts a line after a blank one (as PoB's cleaning assumes), and no tag is left unclosed.

## Bundle evidence

`local/b2card/evidence.sh`, from clean commit `609e7f6`, on the tree data run's extract (no new tables are needed: `Tags.Name` and the item type files were already extracted):

- **`bundle`** twice, byte-identical; `bundle-check` no findings. Against the tree data run's bundle (`local/tree-data/bundle-poe1-a`) only `bundle.json`, `data/item_rules.json` (the 27 `mod_tag_names` gone), and `data/tags.json` (20 monster labels replaced by the 25 tag names) differ.
- **`data-parity`** against PoB's export at format 7 (`local/tree-data/pob-data-1.json`): 557 differences (not-carried 163, patch-delta 130, pob-derivation 184, representation 80, `uncategorized` 0), 314 overlay rows, 0 failed rules; the tree data run's counts.
- **`tree-layout`** (export 3.29.1, `8bd138b`) on the new bundle; `bundle-check` clean. Against the tree data run's layout bundle, the same three files differ.
- **`calc-run`** on the tree data run's layout bundle against the new one, and the new one twice: public 136 builds (135 ok; the one failure is `malformed.xml`, `IMPORT_XML`, by design) and corpus 21 builds, all documents identical before and after and between the two new runs. **No calculation changed.**
- **Game truth:** the three tests in `crates/loom-app/tests/game_truth_affixes.rs` pass on the new bundle (`LOOM_B2_BUNDLE`), Urchin's `Life, Defences, Armour` and the five other headers now from the game's table.

The app bundle (layout and sprites) is in the maintainer's OneDrive folder `Exiled-Studio-bundles` as `bundle-poe1-26530219-schema4-card.zip` (8.1 MB, `bundle-poe1-tree-a/`, 86 files), SHA-256 `4ebd337bb43cceaec6a75d68b0a183ebf5504d8fa05ca8e6997c326bc78d336f`, with a `.sha256` file beside it (checked there with `sha256sum -c`); its `bundle.json` SHA-256 is `bf5704a2db5e1448f2e29572f543dcb378b2532ddfe6dc8cb0baa8813f9f497a`. It replaces `bundle-poe1-26530219-schema4-tree.zip`. Not committed: it carries GGG's sprite sheets (ADR-000, ADR-005 Amendment 2).

## Flavour text evidence

`local/b2card/evidence2.sh`, from clean commit `d2f5f54` (the code of `8168fcd` with its docs):

- **`extract`** of build 26530219 twice, byte-identical (`dirty: no`): 103 tables, all Ok, dangling keys 0. Against the tree data run's extract, `UniqueStashLayout` and `UniqueStashTypes` are new and the manifest differs.
- **`bundle`** twice, byte-identical; `bundle-check` no findings; manifest SHA-256 `7f6aba0539cda9a2cd2b702fbf31730fbc623be57439ad41ae480440d14c267e`. Against the tag-name bundle above, `poe1/unique_flavour.json` is new (1,332 rows, 1,319 names, 247 KB) and `bundle.json` differs.
- **`data-parity`** against PoB's export at format 8 (twice byte-identical): table `flavour_text` 1,356 PoB rows, every shared row's lines equal, 56 differences under one rule (`docs/phase-3/data-parity.md`, "B-2: table `flavour_text`"), 32 bundle-only rows; totals 613 (`uncategorized` 0), 0 failed rules, 370 overlay rows. Summary: `data-parity-poe1-26530219-flavour.json`.
- **`tree-layout`** on it, `bundle-check` clean; against the tag-name layout bundle only `poe1/unique_flavour.json` and `bundle.json` differ.
- **`calc-run`** before and after (public 136, corpus 21), and after twice: every document identical. **No calculation changed.**
- **Game truth:** the three tests in `crates/loom-app/tests/game_truth_affixes.rs` pass on it. Spot checks in the file: Kaom's Heart, Goldrim, Headhunter as traced above; Kaom's Way `FatedUnique1`; Tabula Rasa and Voidheart none.

The app bundle is in OneDrive `Exiled-Studio-bundles` as `bundle-poe1-26530219-schema4-flavour.zip` (8.2 MB, `bundle-poe1-tree-a/`), SHA-256 `a9b9f18c795118142c4c99fca533ac06baa8c4190e44fbc61105dfbd905e7d13`, with its `.sha256` file (checked there); its `bundle.json` SHA-256 is `7dfab3baace2dde6d9fe0afc3bd7030f063013de16d20c17634083b061d0c9f3`. It replaces `bundle-poe1-26530219-schema4-card.zip`. Not committed (GGG's sprite sheets).

## For the maintainer to decide

1. **Flavour text in bundles:** decided 2026-11-15, "add flavour text" (shipped above). Open: pairs for the 28 upgraded versions without a text, from in-game evidence.
2. **Socket caps from the data** (maintainer 2026-11-15: another session's work). The item type files give them by base and item level, so no hard-coded table is needed. A follow-up PR would add a bundle file (each base's `socket_info` through its chain) and the card's "maximum sockets"; it would follow the game, not PoB, for Gnarled Branch, Corroded Blade, and items under item level 50 (display only; that PR would record the difference with this page as evidence, in the divergence register if the maintainer wants display differences there). **Code done in B-2 item evidence (E9, `item-evidence.md`)**: the bundle file `poe1/socket_caps.json` and the card's `max_sockets`. The real file needs the main workstation's next bundle build.

Still open: weapon and armour values checked against the game need the maintainer's in-game copies and Alt screenshots (`ux-improvements-notes.md`, "Maintainer tasks").
