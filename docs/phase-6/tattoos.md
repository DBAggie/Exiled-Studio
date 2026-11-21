# Tattoos, runegrafts, and other overrides (P6-11av)

Found by P6-10a part 3 (`docs/phase-6/timeless.md`, "Part 3").
- **Where they are:** a spec's `<Overrides>` replace a node's lines with a tattoo's, a runegraft's, or a keystone tattoo's. Tattoos are in 23 of the 28 community builds and 9 of the 21 corpus builds.
- **Before this step:** the XML reader kept `<Overrides>` as read (a round trip writes it back), and nothing applied it. A tattooed node counted its own stats.
- **How this showed:** Cyclone and Kinetic Fusillade were farther from PoB in Strength once their Lethal Pride was read. Several of their tattoos sit on +10 Strength nodes.

## The data

**Extraction:**
- The new PoE1 roots `TATTOO_TABLES_POE1` (`PassiveSkillOverrides`, `PassiveSkillTattoos`; `tools/extractor/src/extract.rs`). The tables they reference come with them (`PassiveSkillOverrideTypes`, `PassiveOverrideLimits`, `PassiveSkillTattooTargetSets`).
- The extract is `local/extract/poe1/26530219-tt`, from build 26530219's full depot, with the same game index as the evidence bundles.

**The bundle file** `poe1/tattoos.json` (`loom_data::bundle::tattoos`, written by `bundle`, `tools/parity-cli/src/tattoos.rs`), as PoB's `Export/Scripts/tattooPassives.lua` reads the same table:
- **One entry per `PassiveSkillOverrides` row (343):** its ID, name, type, stats (`Stats` paired with `StatValues`), the passive a keystone tattoo grants (`AllocatedPassiveSkill`), and whether it grants a skill (`Effect`).
- **A keystone tattoo** carries its passive's name and stats. PoB's exporter reads the passive's row for it, so a spec names it by the keystone's name.
- **A name used twice:** the last row wins, as PoB keys its table by name in the table's order.

**The validator:** every stat exists, every keystone tattoo's passive is one of the bundle's passives, and each ID is on one row.

**PoB's export, format 16** (`tools/oracle/data/pob_data.lua`), adds `tattoos`: `Data/TattooPassives.lua`'s nodes by name, each with its ID, override type, keystone flag, and stats' ranges.

**Data parity, table `tattoos`** (`tools/parity-cli/src/data_parity/tattoos.rs`), by name:
- **Compared:** the ID, the override type, the keystone flag, and each stat.
- **On the evidence bundle `b-tt`:** 907 values compared, 22 differences, all `representation` under the new rule `tattoo-display-units`.
  - PoB stores each stat in its description's unit: milliseconds as seconds, per minute as per second, per-myriad as a percent, and a negative "reduced" stat as its magnitude.
- **Bundle-only rows (170):** the rows PoB's exporter skips or keys away (empty names, `[DNT ...]`, the `DisplayRandom...` rows, the ascendancy tattoos without a name of their own).
- data-parity `uncategorized` stays 0.

**Evidence bundle** `local/p6/packs/b-tt`: `b-tl` plus `poe1/tattoos.json`. `bundle-check` reports no findings; it is local and unsigned.

## The engine

**Reading the overrides** (`loom_pipeline::overrides`): the spec's opaque `<Overrides>` element, each `<Override>`'s `nodeId` and `dn` (PoB's `PassiveSpec.lua` 148 to 175). The text inside an override is PoB's display, not read.

**Applying them** (`loom_pipeline::tattoo`, `ResolvedPassives::changed`, renamed from `conquered`):
- **An allocated tattooed node** takes the override's stats in place of its own.
- **A reason, never the node's own stats:**
  - a bundle without the file (`tattoo-data`);
  - a name it does not hold (`tattoo`);
  - an override that grants a skill (`tattoo-skill`).
- **A mastery with a runegraft** takes no chosen effect. PoB replaces the effect with the override (`PassiveSpec.lua` 1393 to 1398).
- **With a Timeless jewel's conquest:** PoB applies the tattoo first (`PassiveSpec.lua` 1187), so a conquest that replaces wins and one that adds goes on top of the tattoo (`over_tattoo`).
- **The small passive rule:** a tattooed node counts as an attribute node, as PoB's `node.isTattoo` (1355 to 1364: +2 instead of +4 Strength or Dexterity, 1% instead of 2% Ward, the devotion node instead of +5 Devotion).
- **Tests:** `loom_pipeline::tests::overrides_should_read_each_node_and_name`, `tattoos_should_replace_and_conquests_apply_after_them`.

**Checked against PoB's own nodes:** a local PoB script listing each allocated node PoB marks `isTattoo`. In every public build with tattoos, and every corpus build with them, the tattooed nodes are the same set as here.

## Evidence

**Fixture** `tattoos-ice-nova-witch.xml`: the Timeless fixtures' tree without the jewel, with four allocated small passives tattooed:
- Ngamahu Firewalker, fire resistance;
- Tukohama Shaman, life regeneration per minute;
- Hinekora Storyteller, chaos resistance;
- Kitava Blood Drinker, life leech per-myriad.

PoB applies all four, and all 77 computed fields match.

**Public** (b-tt, against P6-10a part 4's run):
- 0 unexpected; 282 values moved, 108 toward PoB, 174 away, 39 now equal.
- **Kinetic Fusillade:** Strength, Intelligence, and Life now equal PoB's. Its bleed and poison DPS are computed now (partial), from its tattoos' chances.
- **The away moves** come from other gaps, which tattoos no longer hide. The tattooed nodes are PoB's in every build.
  - Cyclone's Strength is now 219 against PoB's 246. Its seven tattooed nodes are PoB's (each a +10 Strength node), so about 27 Strength is missing elsewhere.
  - Lightning Trap's Strength is 89 against 131. Its four tattooed nodes are PoB's.

**Corpus:** 100 values moved, 58 toward PoB, 42 away, 31 now equal; no state changed.

## Left

- The gaps tattoos made visible: Cyclone's and Lightning Trap's Strength, and other builds' attributes and pools.
  - **Lightning Trap:** its Mageblood's catalysed Strength; equal to PoB's since P6-11ax (`docs/phase-6/stat-mop-up.md`).
  - **Cyclone:** its staff's `{custom}` line `+(25-28) to Strength and Intelligence` (27 at its roll), which PoB applies. The importer excludes `{custom}` item lines by Spike B's plan (`special-case/pob-custom`; ADR-015 Amendment 2 covers config custom lines only), so the field stays partial with that reason. Applying them would change that decision; it is the maintainer's.
- Overrides that grant a skill (`tattoo-skill`): none in the public or corpus builds.
