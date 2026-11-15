# B-2 PR 2: the PoE1 bundle rebuilt with the new item rules (2026-11-15)

The maintainer task PR #112 left (`dev/active/ux-improvements/ux-improvements-notes.md`, "Maintainer tasks"): rebuild the PoE1 bundle so `data/item_rules.json` carries `ModRules.affix_generations` and `TextRules.mod_tag_names`. Run on the main workstation at `f90f5cd` (branch `feat/b2-inferred-mods`). No data schema bump; no calculation reads these rules.

## Inputs
- **Extract:** the B-2 data run's extract of Steam build **26530219** (`local/b2/extract-26530219`, the depot download described in `README.md`), not re-extracted: the installed game is a newer build the extractor does not read.
- **Tool:** `cargo build --release --locked -p loom-parity-cli`, then `loom-parity-cli bundle local/b2/extract-26530219 --out local/b2/bundle-poe1-rules-a` and the same into `bundle-poe1-rules-b`.

## Result
- **Two runs byte-identical:** `diff -r -q` of `bundle-poe1-rules-a` and `bundle-poe1-rules-b` reports nothing.
- **Manifest SHA-256:** `4cdd0bdbb752bdd62f0cc4ad1216d50fa4af45dd093df69960f8156d8a8d757f` (data schema 4, format 2, `steam_build` 26530219). The B-2 data run's was `32d3d74a9deac8c8d3b279d7d578145f4503b6e5e3c414f32ad6906eaf2efff2`.
- **`bundle-check`:** no findings (`{"findings": []}`, exit 0).
- **Against the B-2 data run's schema 4 bundle** (`local/b2/bundle-poe1-a`), only two files differ:
  - `bundle.json`: only the `data/item_rules.json` entry of `files` (SHA-256 `0f9db5f9…` to `beeaaf23…`, 6,376 to 7,936 bytes). `source` and the other keys are unchanged.
  - `data/item_rules.json`: two keys added, nothing else changed. `mods.affix_generations` is `{"prefix": [1], "suffix": [2]}`. `text.mod_tag_names` has 27 pairs: life, mana, defences, armour, evasion, energy_shield, damage, physical, elemental, fire, cold, lightning, chaos, resistance, attribute, attack, caster, speed, critical, minion, ailment, bleed, poison, aura, curse, gem, block.
- **Game-truth tests:** `LOOM_B2_BUNDLE=local/b2/bundle-poe1-rules-a cargo test -p loom-app --test game_truth_affixes -- --ignored`: 3 passed, 0 failed (`the_named_helmet_should_head_every_mod_with_its_tier`, `the_exported_helmet_should_head_only_the_mods_its_lines_allow_one_reading_of`, `the_headers_should_name_the_tags_the_game_shows`).
- **Public fixtures `calc-run`** (the B-2 data run's command, `--scope docs/phase-3/calc-scope-poe1.json`): 136 builds, 135 ok, 1 failed (`malformed.xml`, `IMPORT_XML`, the deliberate bad fixture, as before); field counts equal to the B-2 run's (2,270 complete, 176 partial, 482 unsupported, 312 indeterminate). **Every calc document is identical** to the B-2 data run's (`diff -r -q` of the `documents` folders). The summary (`rules-public-calc-run-summary.json`) differs from `b2-public-calc-run-summary.json` only in the tool commit, the bundle's manifest SHA-256, and the timing figures.

## The bundle for the other workstation
`bundle-poe1-26530219-schema4-rules.zip` (5.2 MB, 77 files under `bundle-poe1-rules-a/`), SHA-256 `6e49f0157248fbf1ce2b2bfa2ecbc2a8c45a94811df80de79a3b6d8fdf666e40`, in the maintainer's OneDrive folder `Exiled-Studio-bundles` beside a `.sha256` file (checked there with `sha256sum -c`). It replaces `bundle-poe1-26530219-schema4.zip` for B-2 PR 2 onward; unzip it, then run `bundle-check` on `bundle-poe1-rules-a`. The bundle is not committed (ADR-000).
