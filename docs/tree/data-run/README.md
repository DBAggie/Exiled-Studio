# Tree data run (2026-11-15)

The maintainer-run work PR #113 left, on the main workstation, from commit `d87bc0e` with a clean tree (script `local/tree-data/evidence.sh`, local only). Method and findings: [editor.md](../editor.md), "The real radii".

- `data-parity-poe1-26530219.json`: data-parity of the new bundle (Steam build 26530219, `poe1/jewel_radii.json` added) against PoB v2.67.2's export at format 7. `jewel_radii`: 15 values, 0 differences; `uncategorized` 0.
- `public-calc-run-summary.json`, `corpus-calc-run-summary.json`: `calc-run` of the public fixtures and the corpus on the new bundle with the tree layout (`tree-layout`, export 3.29.1).
- `public-vs-rules.json`: what changes against the rules bundle with the layout and no radii. Only the community Essence Drain build changes; no corpus document changes; two runs on the new bundle are byte-identical.
