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
