# Phase 2, step 2: lift evidence

The four lift PRs (P2-2a to P2-2d) move the item import pipeline from `tools/parity-cli` into the core crates without changing behaviour (`dev/active/phase-2-canonical-model/phase-2-canonical-model-plan.md`, step 2). Each PR runs `resolve-corpus` on Spike B's inputs (the maintainer's 21 builds, the PoE1 extract `local/determinism/poe1/run1` of Steam build 26530219, the six PoB tags) before and after its change, from a clean tree, and compares the step 2 field list:

```
cargo run --release -p loom-parity-cli -- resolve-corpus "<builds-dir>" local/determinism/poe1/run1 --out <dir> \
  --pob v2.4.0=local/pob-v2.4.0 --pob v2.21.1=local/pob-v2.21.1 --pob v2.35.3=local/pob-v2.35.3 \
  --pob v2.48.0=local/pob-v2.48.0 --pob v2.57.0=local/pob-v2.57.0 --pob v2.67.2=local/pob-v2.67.2
python -I tools/parity-cli/scripts/compare-spike-b.py <before-dir> <after-dir>
```

The script compares `non_unique`, `unique`, `rate`, `resolved_kinds`, `adapted`, `ambiguous`, `categories`, `mod_backed`, `items`, `items_by_rarity`, `items_by_scope`, `line_kinds`. It also reports any other summary key that differs and whether `report.json` is byte-identical. Only `inputs.tool` (the commit) may differ. The outputs stay in `local/` (ADR-007). The summary is scrubbed, but it is the same file Spike B committed, so only the verdict and hashes are recorded here.

## P2-2a: item input (`pob_item` move, `to_domain`), 2026-11-10
- Before: `c7bbffa` (P2-1 tip), clean, exit 0.
- After: `916640a` (the move), and again at `e7ffda1` (review fixes), each clean with exit 0. Both runs give the same result.
- All 12 fields are equal. The only other key that differs is `inputs` (`tool.commit`). `report.json` is byte-identical: SHA-256 `eda17243a5af1a7f…` before and after.
- The after run also equals the committed Spike B summary `docs/spikes/spike-b/resolve-corpus-poe1-26530219.json` on all 12 fields.
- How to run it: with uncommitted doc edits, the tree reads dirty, `gate.inputs_recorded` turns false and the run exits 1. Stash the docs for the run (`git stash push -u`), or run it before writing them.

## P2-2b: scope and bases (`ItemData`, `ItemRules`, `find_base`), 2026-11-10
- Before: the P2-2a baseline above (`c7bbffa`; P2-2a left the result unchanged).
- After: `814a896`, and again at `591aa43` (review fixes), each clean with exit 0.
- All 12 fields are equal. The only other key that differs is `inputs` (`tool.commit`). `report.json` is byte-identical.

## P2-2c: Mods filter (`import::mods`, `ModRules`), 2026-11-10
- Before: the P2-2a baseline above (`c7bbffa`).
- After: `f03a92c`, and again at `54d6fb6` (review fixes), each clean with exit 0.
- All 12 fields are equal. The only other key that differs is `inputs` (`tool.commit`). `report.json` is byte-identical.

## P2-2d: compositions, grafts, properties, tolerances, 2026-11-10
- Before: the P2-2a baseline above (`c7bbffa`).
- After: `4c22ec2`, and again at `0222f0c` (review fixes), each clean with exit 0.
- All 12 fields are equal. The only other key that differs is `inputs` (`tool.commit`). `report.json` is byte-identical.

## Step 2 result
After the four lift PRs, the whole item pipeline below the categories runs from `loom-parser::import` over the domain item, with its per-game rules as `ItemRules` data. On Spike B's inputs it gives exactly Spike B's result: the 12 fields are equal and `report.json` is byte-identical. The categories, the PoB legacy-text evidence, and table loading stay in the tool, as planned.

## P2-6a: the corpus through the bundle (`resolve-corpus --bundle`), 2026-11-10
Plan step 6 and gate 2 ask that the corpus resolution through the data bundle, not the extract, give Spike B's field list. Same inputs as above, plus the PoE1 bundle that `loom-parity-cli bundle` made from that extract (`bundle.json` SHA-256 `6c628cd6…`, `docs/phase-2/bundle-poe1-26530219.json`):

```
cargo run --release -p loom-parity-cli -- resolve-corpus "<builds-dir>" local/determinism/poe1/run1 --out <dir> \
  --bundle <bundle-dir> --pob ...   # the six --pob tags above
```

- Run at `0e10753`, clean, exit 0, in both modes (extract and `--bundle`); `--bundle` again at `8dca976` (review fixes), same result.
- **`--bundle` against Spike B** (`local/p2/spikeb`, the committed summary's run): all 12 fields are equal. No other summary key differs except `inputs` (which records the bundle). `report.json` is byte-identical: SHA-256 `eda17243a5af1a7f…`.
- Extract mode at the same commit against the P2-2d final run: all 12 fields equal, `report.json` byte-identical (the tool change left extract mode unchanged).
- **`bundle-equal`** on the same extract and bundle, at `8dca976`: 40,355 mod rows and 4,309 base names; mod rows, local stats, essence rows, base rows, granted skills, properties, and compositions equal; no reference skipped; 0 mismatches (`docs/phase-2/bundle-equal-poe1-26530219.json`).

## P2-6b: `resolve_build` against the harness, per line, 2026-11-10
With `--bundle`, `resolve-corpus` also resolves every item with the core's `loom_parser::import::Resolver` (what `resolve_build` runs per item), with the base named by `pob_item::item_base`. It compares each modifier line's domain resolution with the harness's outcome before the categories, which are tool-only evidence. The key is the outcome, plus the stats and values of a stat resolution, the `kind` of a typed one, the kept candidates of an ambiguous one, and the tolerance. A continuation takes its first line's key. The result goes to `resolve-build-check.json`, and any difference makes the run exit 1. Same inputs and command as P2-6a, with the same bundle (`bundle.json` SHA-256 `6c628cd6…`).

- Run at `b4c8af0`, clean, exit 0, and again at `cfa90cc` after the review fixes (the cross-check now compares the no-base reason), with the same result.
- **`resolve_build` against the harness:** 6,061 of 6,061 modifier lines are equal, with 0 differences. The domain outcomes are resolved 4,193 (continuations counted with their first line), excluded 1,471, no-template 350, ambiguous 41, no-variant-fits 5, and only-shadowed 1. The corpus has no item without a base.
- **Against Spike B:** all 12 fields are equal. No other summary key differs except `inputs`. `report.json` is byte-identical: SHA-256 `eda17243a5af1a7f…`.
- Extract mode at the same commit: all 12 fields are equal, `report.json` is byte-identical, and no check file is written. `Bases::find` now calls `pob_item::item_base`, the same rules moved.
