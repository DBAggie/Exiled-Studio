# P6-4b: flasks

Phase 6, step P6-4b: active flasks act on the calculation as PoB v2.67.2 merges them (`Modules/CalcPerform.lua` 1602 to 1790). Before this step, an active flask was a reason (`unmodelled-item`, `active-flask`). Plan decision 8: the data comes first, with its `data-parity` table.

## The data

1. **Extractor:** `Flasks` is a PoE1 root (`tools/extractor/src/extract.rs`, `FLASK_TABLES_POE1`). `BuffDefinitions` comes with it.
2. **Bundle:** `poe1/flasks.json` (`loom_data::bundle::poe1::Flask`; converter `tools/parity-cli/src/flasks.rs`). It reads the row as PoB's exporter reads it (`src/Export/Scripts/bases.lua` 247 to 270):
   - the base item's ID and name;
   - life and mana per use;
   - the duration in tenths of a second;
   - the buff's ID;
   - the buff's stats: its `StatsKeys` paired in order with the first value list (`BuffStatValues`), then its `GrantedFlags` at 1 (Silver Flask's `silver_flask_display_onslaught`).
   The file is additive and optional; the loader does not require it.
3. **Validator:** each flask's base is a base item and each stat is a stat (`BUNDLE_DANGLING_REF`), the bases are unique (`BUNDLE_DUPLICATE_ID`), and the values are in range (`BUNDLE_VALUE_RANGE`). A synthetic row and a mutation test cover these checks.
4. **`data-parity`, table `flasks`** (export format 11 adds `flasks`, each flask base's `flask` data): compared by name, the bundle's first row of a name (the game's legacy rows repeat three life flask names, and PoB's values are the first row's):
   - life and mana per use;
   - the duration in seconds;
   - the buff's lines: the bundle's stats rendered together with `tincture_stat_descriptions` (PoB's exporter's file), compared sorted.

   **48 flasks, 192 values, 0 differences.** The other tables are unchanged. Of the 967 differences, 353 are uncategorized, all `cluster_jewels`: the evidence bundle has no cluster file (P6-3). The PoB skill table regenerated from export format 11 is byte-identical.

**The evidence bundle:** `local/p6/packs/b-flasks` is the P6-0b evidence bundle with only `poe1/flasks.json` added. That file comes from an extraction of build 26530219 with the new root. `bundle-check` reports no findings. The bundle is local only and unsigned. The maintainer's next data run ships the file.

## The calculation

`loom_calc::poe1::flasks`, called after the passives, items, and config, before the buffs:

1. **Which flasks:** each flask slot the active item set marks active. Mageblood's magic utility flasks come from unmapped stats, so they are reasons.
2. **The effect:** `scale = 1 + (inc / 100)`, where `inc` sums:
   - the player's `FlaskEffect` INC (`flask_effect_+%`, now mapped);
   - the flask's own effect (`local_flask_effect_+%`);
   - for a magic flask that recovers neither life nor mana, `MagicUtilityFlaskEffect` INC (`magic_utility_flask_effect_+%`, now mapped).
   A conditional `FlaskEffect` is a reason.
3. **Scaling:** the base's buff and the flask's own lines are scaled as PoB's `ScaleAddList` (`skill_modifiers::scale`).
4. **Merging** (`mergeBuff`):
   - the base's buff is applied once per base, at the largest scale among that base's active flasks;
   - two active flasks with one merge key (a unique's name, or the same lines) are a reason.
5. **Silver Flask:** Onslaught is `floor(20 x scale)`, at the largest Silver scale. `buffs` adds the constant 20 for the flag; `flasks` adds the difference, for attacks and for casts.

**The flask's own lines** go through `FlaskMapping`:

| Stats | Reading |
|---|---|
| `local_flask_effect_+%` | The scale. |
| Charges: `local_charges_added_+%`, `local_charges_used_+%`, `local_extra_max_charges` | `NoEffect`. They are read into the item's flask data only, which no calc output reads but through modifiers (`CalcPerform.lua` 1753; `CalcSetup.lua` 938). |
| A utility flask's duration | `NoEffect`. It is read only for recovery (`CalcPerform.lua` 1627). |
| A recovery flask's duration | Unmapped until recovery is computed (P6-10b). |
| Eight "during Effect" stats (`flasks::ALIASES`): armour, evasion, attack and cast speed, crit chance, resistances, a unique's resistances and maximum resistances | Read as the global stat. PoB tags "during Effect" with `Condition:UsingFlask`, which holds while the flask is active (`ModParser.lua` 1737). |
| The consume stats a flask line's description pairs with its effect, at 0 | Dropped. The description's variant pins them to 0. |
| Every other local stat | Unmapped, so a reason (decision 9: most are read into outputs not computed yet: recovery, movement speed, avoidance). |

**New mapping rows:**
- `basalt_flask_armour_+%_final`: `Armour` MORE ("20% more Armour").
- `stibnite_flask_evasion_rating_+%_final`: `Evasion` MORE.
- `silver_flask_display_onslaught`: the `Onslaught` flag.
- `phase_through_objects`: Quartz's Phasing, `NoEffect` (a condition only modifiers and The Hidden Blade's trigger read).

**Not modelled:**
- the `UsingFlask` conditions (unsettled, so a modifier that reads them is a reason);
- recovery (P6-10b);
- flasks applied to minions;
- Iron Flask's unbroken ward.

## Evidence

**Fixture** `smite-flasks-templar.xml`: five active flasks (magic Granite with its own effect and Armour during Effect, Ruby, magic Silver with its own effect, Diamond, Basalt), with flasks and magic utility flasks at increased effect. Every field is `Complete` and equal to PoB but for the weapon's rate (Q11, DIV-0097 to DIV-0099).

**Public `calc-parity`:** 153 builds, 2,572 match (+21), 99 expected differences, 0 unexpected.

**Corpus:** no unexpected difference. Both runs were byte-identical.

**Reasons:**
- `active-flask` was a reason in 27 builds and is now in none.
- 29 builds name unmapped flask stats (74 distinct): recovery, movement speed, ailment avoidance, and the Instilling and consecrated-ground lines. Each is listed for P6-10b and P6-11.

## The P6-4a review, applied here

See `docs/phase-6/config-inputs.md`, "Guards":
- a test checks that no `NAMED_ONLY` name is a condition the engine settles;
- a test checks the classified inputs through `input_use`;
- `DEFAULT_ON`;
- the corrected counts (59 inputs added, 89 entries, 36 inputs left);
- the readers the review found (`CalcDefence.lua` 2216 and 2238, `CalcPerform.lua` 1469).
