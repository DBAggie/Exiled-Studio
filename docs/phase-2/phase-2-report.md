# Phase 2 report: canonical model and data

Phase 2 of `docs/plan/ACTION-PLAN.md` ("Canonical model and data"), planned in `dev/completed/phase-2-canonical-model/` (plan, context, tasks; PR #41, merged 2026-11-10). Report written 2026-11-11 (P2-11).

**Verdict: the gate is met.** Every gate item below holds on the recorded inputs. CI checks items 1a, 2a, 3b, 3c, 3d, 4, 5 (DTO), and 6 on every PR (3c on Linux; the rest on Windows and Linux); at the stack's tip, CI on #57 at `3982838` (run 43031816939) is green, every job. The maintainer-run items (1b, 2b, 2c, 3a, 5 (bundles)) were re-run at `3982838`, the tip of the PR stack (P2-10), from a clean tree on 2026-11-11, and each equals the record its PR committed. The maintainer confirms the verdict by merging the stack.

## 1. What was built
Ten steps in 15 PRs, stacked (maintainer decision 2026-11-10: each PR targets the previous PR's branch; the maintainer merges newest to oldest).

| PR | Step | What |
|---|---|---|
| #42 P2-1 | 1 | `loom-domain`: the `Build` aggregate with a game tag and game-specific sections; typed character, passives, skills, items, config; opaque, ordered containers for everything else; item lines typed by kind; ADR-015 Amendment 1 resolutions (`stats`, typed `kind`, unresolved with a reason, `adapted`). |
| #43 P2-2a | 2a | `pob_item` moved to `loom-serialization::legacy`; the pipeline takes the domain item. |
| #44 P2-2b | 2b | `loom-parser::import::ItemData`, item rules, base lookup. |
| #45 P2-2c | 2c | The Spike B `Mods` filter in `loom-parser::import::mods`, every PoE1 number moved to per-game `ModRules` data. |
| #46 P2-2d | 2d | Granted skills, properties, tolerances (`case_folded`, `split_section`); step 2 complete: the lifted pipeline gives exactly Spike B's result. |
| #47 P2-3a | 3a | `loom-data` bundle format v1: manifest, `BundleSource`, canonical types, loader with `BUNDLE_*` codes, the optional `poe1` section. |
| #48 P2-3b | 3b | `loom-parity-cli bundle`: extract to bundle, PoE1 and PoE2. |
| #49 P2-4 | 4 | `loom_data::bundle::validate` and `bundle-check`: structural, schema, referential, domain, and smoke checks. |
| #50 P2-5a | 5a | `loom_serialization::legacy::pob_xml`: import, semantic export, `xml_equal` (parsed trees, two named normalizations), structure typed, the rest opaque in place. |
| #51 P2-5b | 5b | `<Items>` typed: item text, item sets, slots. |
| #52 P2-6a | 6a | `BundleData`: the item pipeline's data from a validated bundle. |
| #53 P2-6b | 6b | `resolve_build`: every pending modifier line of a `Build` gets its resolution; the build records its bundle (`DataVersion`). |
| #54 P2-7 | 7 | The build document v1 (`loom_serialization::dto::v1`), deterministic JSON; JSON Schema and TypeScript generated from the DTOs. |
| #55 P2-8 | 8 | `build-round-trip`, the gate harness, and the required CI `round-trip` job. |
| #56 P2-9 | 9 | `loom_serialization::migrate` (sequential migrations, report, `MIGRATE_*`); `loom-persistence` (atomic save, migrating load, `PERSIST_*`). |
| #57 P2-10 | 10 | `legacy::share_code`: PoB share codes through the XML adapter; PoB-made codes from the oracle runtime. |

Each PR's decisions and review are in the context file (`dev/completed/phase-2-canonical-model/phase-2-canonical-model-context.md`, "Decisions in P2-…" and "code-architecture-reviewer (P2-…)").

## 2. Inputs
- **Corpus:** the maintainer's 21 PoB builds (local only, ADR-007; never committed; summaries name builds by SHA-256).
- **Public fixtures:** `tools/oracle/fixtures/` (ours) and `tools/oracle/fixtures/community/` (21 credited community builds, ADR-007 Amendment 1): 25 files, 24 of which import (`malformed.xml` is meant not to).
- **Game data:** PoE1 Steam build 26530219 and PoE2 Steam build 26476662 (the Spike A and B builds), extracted to `local/determinism/<game>/run1` (format 4), converted to bundles: PoE1 `bundle.json` SHA-256 `6c628cd6…`, PoE2 `366342d2…` (`docs/phase-2/bundle-poe*-*.json`).
- **Synthetic bundles and builds** (in CI): `crates/loom-data/tests/fixtures/bundle-poe1`, `bundle-poe2` (no PoE1 tables); `tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml`, `synthetic-poe2.json`.
- **Oracle:** PoB v2.67.2 (`b32759ab`), pinned in `tools/oracle/manifest.json` (Spike C).
- **Legacy text evidence for `resolve-corpus`:** six PoB tags, `git archive` of `src/Data` (Spike B).

## 3. Gate verdict
The gate is the plan's "Gate (objective, checkable)". Commands are run from the repository root; `<builds-dir>` is the corpus folder.

### 1. Semantic round-trip: met
- **(a) every public fixture, in CI.** Job `build round-trip` (Windows and Linux, required in `ci-ok`): `build-round-trip tools/oracle/fixtures tools/oracle/fixtures/community --expect-import-error malformed.xml`. 25 inputs: (a) to (d) pass for 24; `malformed.xml` gives `IMPORT_XML` as expected (`docs/phase-2/build-round-trip.md`). These runs have no bundle: in CI, line resolution is exercised on the synthetic builds (item 3b), and on real builds only by the maintainer-run corpus run (b).
- **(b) the 21 corpus builds with the PoE1 bundle.**
  ```
  cargo run --release -p loom-parity-cli -- build-round-trip "<builds-dir>" --bundle <poe1-bundle> --anonymous --out <dir>
  ```
  **21 of 21 pass (a) to (d)**: 1,055 items, 18,842 source lines, 6,061 modifier lines (4,121 resolved, 23 adapted, 49 continuations, 397 unresolved, 1,471 excluded). First run at `d1aa9b7` (P2-8, summary `docs/phase-2/build-round-trip-poe1-26530219.json`); re-run twice at `3982838` (2026-11-11): exit 0, the two runs byte-identical (`summary.json` and `report.json`), and the summary equal to the committed one except `inputs.tool.commit`.
- **JSON bytes across platforms:** CI `dto cross-platform` compares each document's SHA-256 from Windows and Linux, for the public fixtures (`dto-hashes`) and for the `build-round-trip` runs, the resolved synthetic builds included.
- **Share codes import to the same Build as their XML (step 10):** `crates/loom-serialization/tests/share_code_fixtures.rs`. Our code of each of the 24 fixtures imports to the fixture's build. Two codes made by PoB v2.67.2 on the oracle runtime (`tools/oracle/fixtures/share-codes/`) decode to the exact XML PoB made them from and import to its build.

### 2. Unknown content preserved: met
- **(a) the mutation set, in CI.** Inside `build-round-trip` (`tools/parity-cli/src/build_round_trip/mutate.rs`): on the first element of every distinct element path, an unknown attribute, unknown first and last children (an unknown section on the root), a comment before, a processing instruction after, an unknown element between typed siblings, and an extra earlier item text. Every copy imports, reads back from JSON to an equal build, and exports to a document `xml_equal` to the copy. Public fixtures: 4,492 copies, all survive (counted in-session at `d1aa9b7`; CI requires no failure, not the count); corpus: 3,990, all survive (re-run at `3982838`). The check itself can fail: `lossy_export_should_fail_a_mutation`.
- **(b) source lines in equal source lines out, and unresolved lines quarantined with a reason.** Check (d) of `build-round-trip` on the corpus: every item's source lines are the input's, in the imported build, after resolution, after JSON, and after the PoB XML round-trip. Every one of the 397 unresolved lines is kept with its text and an `UnresolvedReason`, which the domain type requires (ADR-015); 16 of 21 builds are `Partial`, 5 `Complete`; nothing is kept whole. (Spike B's 99.7% counts non-unique lines resolved; these 397 are all lines left unresolved, uniques and legacy text included, the same lines Spike B categorized.)
- **(c) the corpus through the bundle gives the Spike B field list.**
  ```
  cargo run --release -p loom-parity-cli -- resolve-corpus "<builds-dir>" local/determinism/poe1/run1 --out <dir> --bundle <poe1-bundle> --pob <the six tags>
  python -I tools/parity-cli/scripts/compare-spike-b.py <spike-b-run> <dir>
  ```
  At `3982838`: all 12 fields equal Spike B's run, no other summary key differs but `inputs`, `report.json` byte-identical; `resolve_build` equals the harness on 6,061 of 6,061 modifier lines (`docs/phase-2/lift-evidence.md`, P2-6a and P2-6b).

### 3. PoE2 bundles validate, with no PoE1 assumptions in shared crates: met
- **(a)** `cargo run --release -p loom-parity-cli -- bundle-check <poe2-bundle>`: exit 0, no findings; the bundle has no `poe1/` section and loads into the same `loom-data` types. Re-run at `3982838` equal to `docs/phase-2/bundle-check-poe2-26476662.json` (and PoE1 to `bundle-check-poe1-26530219.json`).
- **(b) in CI:** the `build round-trip` job runs `synthetic-poe2.json` with `bundle-poe2` (no PoE1 tables): it loads, validates, drives `resolve_build` (resolved lines, none pending), and round-trips through the JSON document; also `synthetic_builds_should_round_trip_resolved` (`tools/parity-cli/src/build_round_trip/tests.rs`) and the outcome tests in `crates/loom-parser/tests/resolve_build.rs`.
- **(c) in CI:** `cargo test -p loom-parser --test no_poe1_identifiers` scans the shared core crates outside `poe1` modules, `legacy`, and `tests.rs` files for the plan's PoE1-only identifiers, as grown by P2-2c and P2-2d; P2-11 added `ModDomain`, the one name of the plan's list it lacked (no shared source used it). It is a substring scan of source text, so it catches names, not PoE1 logic under new names; item 3b covers behaviour.
- **(d) in CI:** `poe1_only_tables_should_be_in_the_extractor_poe1_lists` (`tools/parity-cli/src/bundle.rs`).

### 4. Invalid bundles rejected: met
`crates/loom-data/tests/bundle_validate.rs`, in CI: each case changes one thing in a synthetic bundle and gives exactly its code (missing file, changed byte, unlisted file, dangling reference, duplicate ID, wrong game tag, a `poe1` section in a PoE2 bundle, unknown format, non-empty signatures, a missing client text, a scope that does not build, item rules that do not validate).

### 5. Determinism: met
- **Bundles (maintainer):** `cargo run --release -p loom-parity-cli -- bundle <extract> --out <dir>`, twice per game at `3982838`: byte-identical (`diff -r`), and identical to P2-3b's runs (PoE1 `6c628cd6…`, PoE2 `366342d2…`).
- **DTO and JSON (CI):** golden bytes for synthetic PoE1 and PoE2 builds (`dto::v1` tests), a second run equal on every public fixture (`tests/dto_fixtures.rs`), check (c) of `build-round-trip`, and `dto cross-platform` (Windows against Linux).

### 6. Hygiene: met
In CI on every PR: the `wasm32 core crates` job (`loom-domain`, `loom-data`, `loom-parser`, `loom-calc`, `loom-serialization`), `schema freshness`, and `cargo-deny` (licenses, bans, sources), all required in `ci-ok`.

## 4. Scrubbed summaries
In `docs/phase-2/`: `build-round-trip-poe1-26530219.json` (corpus, builds by SHA-256, counts only), `bundle-poe1-26530219.json`, `bundle-poe2-26476662.json`, `bundle-check-*.json`, `bundle-equal-poe1-26530219.json`; the method notes `build-round-trip.md` and `lift-evidence.md`. The re-run outputs stay in `local/p2/p2-11/` (ADR-007); they equal the committed files except the tool commit, so none was replaced.

## 5. Known limitations
- **Typed coverage is what Phase 3 needs (plan decision 1).** Everything else is opaque and preserved. Passive node, class, and gem IDs are not checked against data (Phase 3).
- **Resolution:** 397 of 6,061 corpus modifier lines stay unresolved, each with its reason (Spike B's categories); 16 of 21 builds are `Partial`.
- **PoE2 build import** (`.build`) is backlog (ADR-005): a PoE2 build enters as a v1 document only, so the PoE2 round-trip has no PoB XML leg.
- **PoE2 data equality** has no extract-side comparison (`bundle-equal` reads PoE1 extracts only); the PoE2 bundle is checked by `bundle-check` and the ignored `LOOM_BUNDLE_DIR` test.
- **Real bundles are not in CI** (Q1 default): CI uses the synthetic bundles; the real ones are maintainer-run with scrubbed summaries. So CI resolves lines only on the synthetic builds; a resolution change on real builds shows in the maintainer's corpus run, not in CI.
- **Bundles** have no compression, container, signatures, or update channel (Phase 5, Q2). The IMPLEMENTATION-PLAN's "invalid bundles are rejected before signing" is met as "rejected by the validator" (item 4); signing comes with Phase 5.
- **Migrations:** only v1 exists; the framework is exercised by test-only chains.
- **Persistence:** saving to a new name is atomic only where the file system has hard links (NTFS, ext4, APFS, WSL 2's `/mnt`; not FAT32 or exFAT); replacing an existing file is not guarded against another process; no lost-update check, stale temporary-file sweep, or Windows rename retry (Phase 4, with autosave and the single-writer build task). No SQLite index (Phase 4, Q3).
- **Share codes:** PoB's own `SaveDB` output varies per run (Lua `pairs` order), so PoB-made codes are committed with their XML; that PoB reads our codes is not shown (possible with the oracle runtime later). Headless PoB has no `Deflate`; the maker uses zlib from PoB's runtime.
- **XML equality** normalizes attribute order and whitespace between elements, nothing else (decision 12; `NORMALIZATIONS`).
- **Oracle:** the `variant_order` shim picks the base variant where PoB v2.67.2 picks a Vaal gem variant by chance (Spike C); the upstream issue draft is still to post.

## 6. Open for the maintainer
- Plan questions Q1 to Q6 were taken on their defaults (context file); confirm or change them.
- Post the PoB issue (`local/upstream-issue-pob-vaal-variant.md`, workstation only).
- Merge the stack #57 down to #42, then into `main`.
- Next: Phase 3 (calculations) per `docs/plan/ACTION-PLAN.md`, which needs its own plan PR.

## 7. Reproduce
The commands are in `AGENTS.md` ("Commands") and in each section above. The local inputs' paths are in `docs/handoff/SUCCESSOR-HANDOFF.md` (Workstation).
