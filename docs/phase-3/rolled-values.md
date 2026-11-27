# P3-8b: rolled values of PoB's ranged lines

Phase 3, step 8 (plan `dev/completed/phase-3-calc-slice/`; base `dev` after #79; decisions in the context file, "Decisions in P3-8b").

## Why

After P3-10, every field of the 7 in-scope builds (3 Ice Nova, 4 Smite) was `indeterminate` for one reason: `unsettled-value`. PoB-crafted lines such as `{range:1}+(16-24) to Dexterity and Intelligence` resolve to an interval (ADR-015; Spike B left the roll to later work), and the engine never turns an interval into a number on its own.

## What

The roll is settled outside the build document, from the host, the same way as P3-7's base names (`CalcOptions::item_bases`). Build document v1 refuses unknown fields, so carrying the roll in it would need a v2 and a migration. The roll is derived data that the host can recompute.

1. **Parser** (`loom-parser`).
   - `RawValue::Ranged` keeps a range's ends (`RangeEnds { from, to }`, in the line's order) only when both of these hold:
     - each end shows exactly one raw value;
     - the stat has no numeric handler other than `negate`.
   - `Resolver::resolve_build_ranged` / `resolve_item_ranged` return the ends per item and line. `resolve_build` and `resolve_item` are unchanged.
   - Only lines that this call resolves get ends.
2. **Adapter** (`loom_serialization::legacy::pob_item::line_values`). It applies PoB v2.67.2's rules, cited in its module docs.
   - **Roll.** A line with `{range:r}` (`r` in [0, 1]) is rolled to `roundSymmetric(from + r x (to - from))`.
     - No roll for:
       - a line without a prefix (PoB would use its user setting `defaultItemAffixQuality`, 0.5 by default);
       - a `value(min-max)` line;
       - any line of an item whose `<ModRange>` entries disagree with its prefixes.
     - A `<ModRange>` entry `k` is matched to the `k`-th modifier line, with a tolerance of 5e-7. When an item has more entries than lines, PoB ignores them all, and so does the adapter.
   - **Scaled lines.** The engine does not model these scalings, so the line is marked `Scaled` instead of being given a value:
     - `catalyst`: lines the catalyst's tags reach, ranged or not, plus untagged ranged lines on a catalysed item;
     - `corrupted-range`;
     - `modifier-magnitude`: every line of an item that has "... Modifier magnitudes" or "... effect of Prefixes/Suffixes".
3. **Calc** (`CalcOptions::item_lines`, `Ingest::with_item_lines`).
   - A `Rolled` line's ranged stats take the host's values. If the stats do not match the resolution, or a value is outside the interval, the line keeps its interval and stays unsettled; it is never dropped. A multi-line reading is decided on its first line together with its continuation lines: one scaled line scales the whole reading, and one roll must hold for all its ranged lines.
   - A `Scaled` line gives the new reason `scaled-line` (calc result v1, additive rule; schema, TypeScript, and golden `every-kind.json` regenerated) and no effect.
   - `calc-run` wires the three steps together.

Before this PR the engine ignored catalyst quality without saying so. No mechanic fixture had a catalyst, so no `Complete` field was wrong; now such a line is a reason.

## Evidence

All runs are from this branch's working tree on the P3-10 inputs:
- bundle `local/p3/p3-1d/bundle-poe1-a`;
- scope `docs/phase-3/calc-scope-poe1.json`;
- overlay `local/p3/p3-1d/parity-1/overlay.json`.

Summaries are in `docs/phase-3/rolled-values/`.

| Run | Result |
|---|---|
| New mechanic fixture `rolled-jewellery-witch.xml` (original). Rolls between the ends, a tie on a "reduced" line (`(5-10)% reduced maximum Mana` at 0.5 is 8% in PoB and here), implicit rolls, and an agreeing `<ModRange>` | Oracle 3/3 identical. `calc-parity`: all 17 targets match PoB, and the breakdowns match |
| `calc-run`, public (82 builds), twice | 81 documents (`malformed.xml` `IMPORT_XML` as expected). Complete 914 (897 + the new fixture's 17), partial 334, indeterminate 312, not computed 384. The two runs are byte-identical |
| The same against P3-10's documents (80 builds) | No `Complete` field changed. 266 fields went from `indeterminate` to `partial`; 111 stay `indeterminate` (tree version, missing data) |
| `calc-run --data-overlay`, public | The same field states |
| `calc-run`, corpus (21, `--anonymous`) | All `indeterminate` (older trees), as in P3-10 |
| `unsettled-value` field reasons, public | 487 to 17 |

The remaining 17 are one build (`lightning-trap-of-sparking-elementalist`), 3 lines of a Watcher's Eye. One of its 108 `<ModRange>` entries (0.743902) maps to a line whose prefix is 0.5, so the adapter leaves the whole item unrolled.

**In-scope builds.** All 7 no longer have any `indeterminate` reason. They are `Partial` on:
- other gems (`unmodelled-gem`: auras, buffs);
- config inputs;
- Alira and the pantheon;
- unmapped stats.

They are not compared yet. No catalyst or magnitude line among their equipped items reaches them. 12 public builds carry `scaled-line`: 41 catalyst lines and 62 magnitude lines.

**Full parity run.** `oracle-run` on the whole manifest: 82 fixtures, every `ok` fixture identical 3/3, exit 0. Then `calc-parity` against the public `calc-run`:
- **No difference.** 914 fields and 354 breakdown terms match PoB.
- **Every mechanic target met:** 880 of 880 over 45 fixtures, the new one included.
- No register entry is unused, and no mechanic fixture is missing.
- Exit 5 (`incomplete`) because 34 real builds have fields that are not `Complete`; P3-10 had the same.
- The corpus is all `indeterminate` (older trees), so nothing is compared.

Summaries are committed in `docs/phase-3/rolled-values/`.

No fixture has a conflicting `<ModRange>`, a catalyst, or a magnitude line among its compared fields, because those cases give reasons, not values; the unit tests cover them.

Tests:
- parser: ends in order, reversed (`reduced`), crossing zero, lossy and scaled ends give none; `resolve_build_ranged` on the synthetic bundle;
- adapter:
  - rolls and ties;
  - no roll without a prefix, out of range, or with a value before the range (PoB's pattern);
  - catalyst lines: tagged, untagged ranged, `{unscalable}`, quality 0, and the unnamed `Quality (... Modifiers)` form;
  - corrupted range;
  - magnitudes, and an unselected magnitude line;
  - `<ModRange>` agreeing, conflicting, too many, and at the 5e-7 tolerance edge;
  - multi-line readings: one roll for all lines, different rolls, or a scaled continuation line;
  - affix spec ranges ignored;
- calc: rolled, mismatched, out-of-range, and scaled lines; end to end through `calculate` (unsettled without the host, rolled and `scaled-line` `Partial` with it);
- calc result v1 golden files.

## Known limits

- **Raw space.** The roll is interpolated on raw values, and only for stats whose raw value is the shown number or its negative. Scaled stats (`divide_by_*`, `per_minute_to_per_second`, ...) get no roll, because PoB's `modScalability` precision is not reproduced.
- **Negative ties.** PoB's fallback path (a line without `modScalability` data) rounds a negative tie up. No fixture line reaches it.
- **ModRange mapping.** `<ModRange>` ids follow PoB's internal lists (buff, enchant, scourge, implicit, explicit, crucible), and the adapter maps them by position. A misaligned entry stops the item's rolls rather than changing a value. An entry that lands, misaligned, on a line without a range is accepted. If PoB would have applied it to a ranged line, that line gets the prefix's roll, not the entry's. Only the parity runs guard this, and it is the one remaining path to a value without a reason.

## Follow-ups

- **Catalyst scaling.** Needs the game's `Stats.IsScalable` in the bundle; PoB's `data.modScalability` comes from it. The game then scales the catalysed lines, and `scaled-line` `catalyst` goes away. *Done in P6-11ax* (`docs/phase-6/stat-mop-up.md`, "P6-11ax"): one-line readings whose stats' scalability the bundle holds; the untagged ranged lines, multi-line readings, and the unnamed catalyst line stay reasons.
- **Modifier magnitudes and corrupted ranges:** the same, with their own scalars.
- **PoB's `<ModRange>` list order**, for uniques with variant lines such as the Watcher's Eye above.

## Commands

`<out>` is a new folder. The commands follow P3-10's in `docs/phase-3/attack-hits.md`, with `tools/oracle/fixtures/fixtures.json` now listing the new fixture.

```sh
lpc=target/release/loom-parity-cli; b=local/p3/p3-1d/bundle-poe1-a; scope=docs/phase-3/calc-scope-poe1.json
public="tools/oracle/fixtures tools/oracle/fixtures/mechanic tools/oracle/fixtures/community local/oracle/pob/spec/TestBuilds/3.13"
$lpc oracle-run tools/oracle/manifest.json local/oracle --out <out>/oracle tools/oracle/fixtures/fixtures.json
$lpc calc-run $public --bundle $b --scope $scope --out <out>/public
$lpc calc-run $public --bundle $b --scope $scope --data-overlay local/p3/p3-1d/parity-1/overlay.json --out <out>/public-overlay
$lpc calc-parity <out>/oracle <out>/public --slice docs/phase-3/slice-fields.json --out <out>/parity --register docs/parity/divergence-register.json --overlay-run <out>/public-overlay --manifest tools/oracle/fixtures/fixtures.json
$lpc calc-run "<corpus>" --anonymous --bundle $b --scope $scope --out <out>/corpus
```

## The agent list (S9, 2026-11-26): an untagged line on a catalysed unique

A catalyst scales a line in PoB by the line's own `{tags}` (`Item.lua` 31 to 62, `getCatalystScalar`), and a unique's lines in a saved build often carry none: Mageblood's "Leftmost (2-4) Magic Utility Flasks...", Nimis' "(25-35)% increased Projectile Damage". PoB leaves such a line unscaled. The game scales a mod by its own tags. So the engine had made every untagged ranged line on a catalysed item a reason (`scaled-line`, `catalyst`): the game might scale it.

**What the game data settles.** The bundle's unique mod rows carry their implicit tags (data schema 3, `ModEntry::implicit_tags`). For an untagged ranged line on a catalysed unique, `loom_pipeline` looks up the unique rows whose stats are the line's reading: the rows of the generation types the item rules fit to a unique's explicit lines, with their stats other than `(0, 0)`.
- **None of them has a catalyst tag:** the game leaves the line unscaled too. Mageblood's flask row has no tags; Headhunter's `damage_+%_vs_rare_monsters` row is `damage`, and Intrinsic scales attributes. Then the line is rolled as any other (`pob_item::line_values_full`), equal to PoB's.
- **A row has one, or no row matches:** the line stays a reason, as before.
- **A rare item's untagged line** is not read this way; it stays a reason.

**Evidence** (the S9 agent: `local/p6/s9-2` on `b-s9`, against `local/p6/s9-1`):
- **Public:** 0 unexpected; `Complete` unchanged (49,371). The `catalyst` reason is gone from every public build: 583 fields lost it (7 builds).
  - Eight values moved, all toward PoB, none away: `elemental-hit-of-the-spectrum-deadeye.xml` and `lightning-trap-of-sparking-elementalist.xml` (Nimis' projectile damage now counts), their average damage, hit DPS, combined and full DPS.
  - The other five builds changed in their reasons only.
- **Corpus:** 0 unexpected; no document changed.
- **Tests:** `pob_item::values::tests::an_untagged_unique_line_should_roll_when_its_rows_lack_the_tags`; the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32.

## The agent list (S9, 2026-11-26): modifier magnitudes PoB does not apply

An item line such as "8% increased Explicit Defence Modifier magnitudes" (a crafted or Heist enchantment, Eyes of the Greatwolf's "are doubled") scales other lines of its item. The engine made every modifier line of such an item a reason (`scaled-line`, `modifier-magnitude`) and left them all out of the calculation.

**PoB applies the magnitudes only to an advanced copy or a crafted item** (`Item.lua` 1377: `if self.advancedCopy or self.crafted`). An item is an advanced copy when a line has a `{range:}` prefix (938 to 940), a `(a-b)` range (932 to 935), or the advanced `{ ` form (551 to 556), and crafted with a `Crafted` spec (768). Otherwise PoB reads each line's printed value. That is also what the game prints: an item's text shows its values with the magnitudes applied.

**The engine now does the same** (`pob_item::values::magnitudes_apply`): on an item that is neither, its lines are read as printed; on an advanced copy or a crafted item they stay the reason.

**A PoB check.** `lacerate-bleed-gladiator.xml`'s axe (Kraken Etcher: "8% increased Explicit Physical Modifier magnitudes" with fixed lines) was run through PoB twice, with and without that line: the same average damage (24,757.0) and the same weapon base (876.5 to 2,279.6). PoB reads the lines as printed.

**Evidence** (the S9 agent: `local/p6/s9-3` on `b-s9`, against `local/p6/s9-2`):
- **Public:** 0 unexpected; `Complete` unchanged (49,371; these builds have other reasons). `modifier-magnitude` went from 410 fields in 5 builds to 165 in 2 (the advanced copies).
  - 77 values moved: 69 toward PoB, 9 of them now equal (Lacerate's armour, fire and chaos resistances, physical damage reduction; Flicker Strike's chaos resistance), and 8 away.
  - **The 8 away, each a field another reason already set apart:**
    - Lacerate's average damage, hit DPS, and leech: 23,006 to 122,577 against PoB's 24,757. The axe's lines now count, and its Deadly Ailments' `support_better_ailments_hit_damage_+%_final` (80% less damage with hits) is unmapped, already a reason on those fields. 122,577 x 0.2 is 24,515. Its configured damage chance moved too.
    - Flicker Strike's energy shield recharge and net regenerations, and RF Chieftain's degeneration, which follow the energy shield and life these items now add.
- **Corpus:** 0 unexpected; 79 values moved, 74 toward PoB (30 now equal), 5 away: two builds' net regeneration and degeneration, by under 0.4 each.
- **Tests:** `pob_item::values::tests::corrupted_range_and_magnitude_lines_should_be_scaled` (an advanced copy and a crafted item scaled, a plain one not); the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32.
