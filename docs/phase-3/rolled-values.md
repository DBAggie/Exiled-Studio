# P3-8b: rolled values of PoB's ranged lines

Phase 3, step 8 (plan `dev/active/phase-3-calc-slice/`; base `dev` after #79; decisions in the context file, "Decisions in P3-8b").

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

- **Catalyst scaling.** Needs the game's `Stats.IsScalable` in the bundle; PoB's `data.modScalability` comes from it. The game then scales the catalysed lines, and `scaled-line` `catalyst` goes away.
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
