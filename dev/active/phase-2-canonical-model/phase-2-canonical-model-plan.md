Last Updated: 2026-11-10 (PR P2-0, #41: plan written, plan-reviewer findings applied)

# Phase 2: canonical model and data (plan)

## Summary
This plan covers Phase 2 of `docs/plan/ACTION-PLAN.md` ("Canonical model and data"), which follows Spikes A, B, and C (all three gates met by 2026-11-10). The ACTION-PLAN gives the scope:
- DTOs and generated schemas;
- the Build aggregate;
- the bundle format and its validator;
- PoB XML import through stat resolution;
- JSON persistence and migrations;
- PoE2 data through the same pipeline (no calcs).

Where the ACTION-PLAN does not contradict `docs/plan/IMPLEMENTATION-PLAN.md` Phase 2, its work packages add detail to this plan.

**Gate (ACTION-PLAN):** semantic round-trip of the corpus; unknown content preserved; PoE2 bundles validate with no PoE1 assumptions in shared crates. The IMPLEMENTATION-PLAN adds two items: invalid bundles are rejected before signing, and DTO-to-domain conversion is deterministic. Both are in the gate below.

Phase 2 does not use the oracle; Phase 3 compares against it. Phase 2 calculates nothing.

## Current state (2026-11-10, `main` at `1f35323`)
- `loom-domain`, `loom-serialization`, `loom-persistence`, `loom-calc`, and `loom-app` hold only crate docs. There is no Build type, no DTO, and no persistence code.
- `loom-data` has `stat_desc` from Spike B (load, render, invert, scope, handlers). It has no canonical game-data model and no bundle type.
- `loom-parser` has two modules:
  - `resolve`, the reverse matcher: `Index::resolve`, `resolve_lines`, `resolve_case_folded`, with outcomes `Resolution`, `Outcome`, `Unresolved`;
  - `pob_item`, which reads PoB item text into typed lines: `read`, `Item`, `Line`, `LineKind`, `Modifier`, `Section`, `Variants`, `magic_base`.
- **The rest of the import pipeline is in `tools/parity-cli`, not in core.** It reads the extractor's table JSON through `extract.rs` (a `Read` closure) and reads PoB XML with `roxmltree`. These parts would move:

  | Part | Where it is |
  |---|---|
  | Base lookup and scope by `ModDomain` (`DOMAIN_SCOPES`, PoE1 values) | `resolve_corpus.rs` (`Bases`) |
  | `Mods` ambiguity filter, with PoE1 `ModGenerationType`, `ModDomains`, and influence constants and the `Essences` columns | `mod_filter.rs` |
  | Item tags | `item_tags.rs` |
  | Composed lines | `categorize.rs` (`COMPOSITIONS`) |
  | Granted skills | `grafts.rs` |
  | Item properties and catalyst quality | `properties.rs` |
  | Split-section marking | `mark_split_lines` |
  | Categories | `categorize.rs` |

  The pipeline takes `pob_item` types as input (`resolve_corpus.rs` imports `Item`, `LineKind`, `Modifier`, `Rarity`, `Section`, `BaseMatch`; `mod_filter.rs` imports `Modifier`, `Section`). This is about 4,000 lines in all.
- `loom-extractor extract` writes `manifest.json` (format 4) and `tables/*.json`, with rows in the shape of the community dat schema. That output is an extract, not a bundle: it has no canonical model, no validator, and no distribution form.
- PoB XML builds:
  - the maintainer's 21 builds (local only, ADR-007);
  - the public fixtures in `tools/oracle/fixtures/fixtures.json`: PoB's five 3.13 builds, our synthetic builds, and the credited community builds (ADR-007 Amendment 1).

  A community build has these top-level sections: `Build` (with `PlayerStat`), `Import`, `Party`, `Tree`, `TreeView`, `Notes`, `Skills`, `Items` (with `ModRange`, `Slot`, `ItemSet`, `SocketIdURL`), `Calcs`, `Config`, `TradeSearchWeights`, `TimelessData`.
- Spike B's results must hold through the move. The fields that must match are listed in step 2.

## Target state
| Crate or tool | What Phase 2 adds |
|---|---|
| `loom-domain` | The Build aggregate (ADR-003). It has a game tag; IDs and value objects; and typed substructures for what Phase 3 needs. Opaque, ordered containers keep everything else. It also has quarantine and completeness states (ADR-014). An item keeps every line it was read with, typed by kind (decision 3). The modifier line follows ADR-015 Amendment 1: stat IDs with values, a typed result per `kind`, or unresolved with a reason, plus the `adapted` tolerance. |
| `loom-data` | Its own canonical game-data model, with game-specific parts in game-tagged sections, and bundle format v1 (manifest plus canonical data files). Bundles load through an abstract `BundleSource` (ADR-016). The bundle validator gives stable error codes. Game-specific item rules (mod domains to scopes, generation types, influences) are data per game, not code constants in shared modules. |
| `loom-parser` | The item import pipeline lifted from `tools/parity-cli`. It takes the domain item as input and reads game data through an `ItemData` trait. One entry point, `resolve_build`, resolves every item line of a Build. |
| `loom-serialization` | Successor JSON DTOs v1, with JSON Schema and TypeScript generated from them; deterministic DTO-domain conversion; sequential migrations with a report. It also holds the PoB legacy adapters: XML import and semantic export, item text (`pob_item` moves here), PoB-only rules, and share codes. Unknown content is preserved. |
| `loom-persistence` | File-first save and load of successor JSON: atomic write, no silent overwrite, migration on load. |
| Tools | `loom-extractor bundle` (extract to canonical bundle); `loom-parity-cli bundle-check` (validator) and `build-round-trip` (the gate harness); `tools/schema-gen` (generates the schemas). |
| CI | Round-trip and unknown-content mutation tests on the public fixtures. Synthetic PoE1 and PoE2 bundles are loaded, validated, and resolved, and invalid bundles are rejected with their codes. A PoE1-identifier check runs on shared code. A cross-OS byte comparison runs. Schemas must be fresh. Every core crate still builds for wasm32. |
| Report | `docs/phase-2/phase-2-report.md`, with the gate verdict and scrubbed corpus summaries in `docs/phase-2/`. |

## Design constraints and decisions
Decisions marked (Q) wait for the maintainer; their defaults are in the context file, "Open questions". The plan proceeds on those defaults.

1. **Type only what Phase 3 needs; keep everything else opaque, in position.**
   - Typed:
     - `Build` attributes: class, ascendancy, level, main socket group, game-specific choices;
     - the active tree spec: tree version, node IDs, mastery effects, jewel sockets;
     - skill sets, socket groups, and gems;
     - items, item sets, and slots;
     - config inputs, as typed key-value pairs (what they mean is Phase 3's concern).
   - Opaque but preserved: `Notes`, `TreeView`, `Calcs`, `Import`, `Party`, `TradeSearchWeights`, `TimelessData`, `PlayerStat` (PoB's cached outputs), `ModRange`, comments and processing instructions, and any element or attribute the adapter does not know.
   - Opaque content keeps its exact position among typed siblings (for example an unknown element between two `<Gem>`s), its name, its attributes in order, and its text. Export writes it back unchanged.
   - An `<Item>` with several text children keeps all of them. The item is read from the last non-blank one, as PoB does (Spike B's rule, in `resolve_corpus.rs`). The other children stay as opaque text in place. Without that, export would drop content (ADR-015).
2. **Typed values keep their source text.** A typed value read from XML keeps the string it was read from: config `number=` values, `nodes` and `masteryEffects` lists, item spec lines, and their order. Export writes the source string while the value is unchanged, so the round-trip does not depend on how floats or lists are formatted. A value the app changes later is rendered fresh. Equality on f64 values compares the source strings, which also settles NaN.
3. **The item model keeps every line.** Each line of an item has a kind: header, spec, modifier, flag, influence, `{custom}`, unrecognized, or a variant line the item does not select. Only modifier lines get a resolution; the other kinds are kept for export and counted. A `split-section` pair is two source lines. The first carries the resolution and the second points to it (`continues: <line index>`). That way "lines in = lines out" counts source lines, and nothing is dropped.
4. **Crate graph: the pipeline takes the domain item, so no new edge is needed.** The XML adapter in `loom-serialization` maps `pob_item`'s output onto the domain item. `pob_item` is a reader of PoB's format, so it moves to `loom-serialization::legacy`. `loom-parser::import` takes `loom_domain` items and an `ItemData`. The dependency graph stays exactly as AGENTS.md has it: `loom-parser` depends on `loom-domain` and `loom-data`, and `loom-serialization` depends on `loom-domain`. Step 1's item types must therefore carry what the pipeline reads: rarity, base name, sections, influences, flags, variant selection, line kinds, and line prefixes. (Q5)
5. **No PoE1 or PoB rules in shared code (ADR-001).**
   - The Build carries `Game` (`Poe1` or `Poe2`). Classes, ascendancies, gems, nodes, and bases are data IDs, never Rust enums. PoE1-only choices (bandit, pantheon) live in `GameSpecific::Poe1 { .. }`.
   - Game-specific item rules are per-game data in the bundle: `ModDomain` to scope, generation types, influence types, and essence columns. The extractor writes them, from the vendored schema's enums, into an `item_rules` section tagged with the game.
   - PoE1-only tables (`Essences`, `AlternateQualityTypes`, `BrequelGrafts`, `BrequelGraftGrantedSkillLevels`, and the PoE1-only lookup tables) sit in the bundle's `poe1` section. The loader and validator never require that section.
   - PoB-only rules stay in `loom-serialization::legacy`: `POB_SUFFIXED_BASES`, the last-text-child rule, and PoB's prefixes.
   - The shared pipeline reads all of this through `ItemData`. CI checks it in two ways (gate 3): a synthetic PoE2 bundle and Build run through the whole pipeline, and a test scans shared code for PoE1-only identifiers.
6. **Item export is generated from the model.** Export builds each item's PoB text from the typed item: every line in order, with source text and prefixes (decision 3). The round-trip compares the re-imported items, so the item model is actually tested rather than passed through as raw text.
7. **Adapted lines (ADR-015 Amendment 1 leaves this choice to Phase 2).** The persisted line carries `adapted: "case-folded" | "split-section"`, or nothing when matching was exact. Export keeps the line's source text whether or not it was adapted: PoB is the export target, and rewriting the user's text to the game's would be a silent change. A line created later in the app has no source text, so it is rendered from its stat description. (Q4)
8. **Bundle v1 is the canonical form, with our own schema.**
   - IMPLEMENTATION-PLAN WP3 asks for an independent canonical schema. Bundle v1 therefore uses our own type and field names. The mapping from dat-schema rows lives in the extractor, so a dat-schema rename changes the converter, not the bundle.
   - Layout: a directory with `bundle.json` and canonical JSON files (sorted keys, stable row order).
   - `bundle.json` records the bundle format version, data schema version, game, Steam build, index SHA-256, extractor commit, decoder and its version, dat-schema revision, and each file's SHA-256 and size. It also reserves a `signatures` field, empty in v1.
   - The validator runs on this form, so an invalid bundle is rejected before any signing exists.
   - The compressed single-file container and real signatures come with the update channel in Phase 5 (ADR-010). (Q2)
9. **Core reads bytes; hosts read files (ADR-016).** `loom-data` defines `BundleSource` with `list()` and `read(path)`. The list is needed for the "no unlisted file" check. The extractor, parity-cli, and later `loom-persistence` implement it over files. The validator hashes with `sha2`, a new `loom-data` dependency (pure Rust, builds for wasm32), which P2-3 checks with `cargo deny`. No compression crate enters a core crate in Phase 2.
10. **Real game data stays local; CI uses synthetic bundles.** CI has no game install. Synthetic PoE1 and PoE2 bundles, made of our own hand-written rows, test the loader, validator, and pipeline in CI. The maintainer builds and checks the real bundles, and commits scrubbed summaries, as in Spikes A and B. (Q1)
11. **Generated schemas.**
    - DTOs derive `schemars::JsonSchema` behind a `schema` feature of `loom-serialization`; only `tools/schema-gen` enables it.
    - `schema-gen` writes `schemas/build.v1.schema.json`. The UI generates TypeScript from it into `ui/src/generated/` with `json-schema-to-typescript` (a pnpm dev dependency).
    - CI regenerates both and fails on any diff.
    - Licenses: `schemars` and `json-schema-to-typescript` are MIT. P2-7 checks again with `cargo deny`.
12. **XML in core.** The adapter reads with `roxmltree` (MIT or Apache-2.0, pure Rust, already a parity-cli dependency) and writes with our own small writer. Round-trip equality compares **parsed trees**, not text: both sides are parsed with `roxmltree`, attributes compared as maps, line endings in text normalized. That makes the XML declaration, whitespace between elements, escaping, and character references (`&#10;` against a literal newline) equal by construction. Any other normalization is a closed list in step 5, each entry with a reason and a test.
13. **Persistence.** File-first: write a temporary file in the same folder, then rename it. Never write over the file a build was imported from. Load runs migrations and returns a migration report. The SQLite index and cache (ADR-009) come with the build list in Phase 4. (Q3)
14. **Migrations.**
    - Sequential `vN -> vN+1` functions over `serde_json::Value`, run before DTO parsing. Each one adds entries to the migration report.
    - Phase 2 ships only v1, so test-only migrations exercise the framework: a synthetic v0 to v1, and a document newer than supported, refused with a typed error.
    - The Build records the data bundle it was made with (game, data schema version, Steam build), so a data update never reinterprets a build silently (ADR-010).
15. **Errors and completeness.** Every failure has a stable code (ADR-014): `BUNDLE_*`, `IMPORT_*`, `MIGRATE_*`, `PERSIST_*`, `DTO_*`.
    - Layers (maintainer decision 2026-11-10, recorded in P2-8): `MIGRATE_*` reads the document's version first on load, so a document newer than supported gives `MIGRATE_TOO_NEW` before it is parsed; `PERSIST_*` covers file I/O and atomic writes; `DTO_*` is a malformed build document, from any source. No layer wraps or renames another layer's code.
    - An import result is `Complete`, `Partial` (some content quarantined), or `Fatal`.
    - `Indeterminate` is a calculation state (ADR-014), so import never produces it.
    - Nothing is dropped silently.
16. **Share codes are in the gate.** ADR-005 makes XML and share codes first-class external adapters. A share code is base64url of zlib-deflated XML. Decoding uses `miniz_oxide` (MIT, Zlib, or Apache-2.0; pure Rust) in `loom-serialization`. (Q6)

## Steps

### 1. Domain model (M)
Build the aggregate in `loom-domain`:
- `Game`; IDs (`StatId`, `NodeId`, `GemId`, `ItemId`, ...); Build; character; `GameSpecific`;
- passive spec; skills; config inputs; typed values with their source text (decision 2);
- items with every line typed by kind (decision 3) and the fields the pipeline reads (decision 4); item sets and slots;
- the modifier line enum (ADR-015 Amendment 1) and `Adapted`;
- the `Opaque` ordered container, with content in place between typed siblings;
- quarantine entries with reason codes, and completeness.
- **Accept:**
  - unit tests build a PoE1 and a PoE2 Build;
  - every line kind, every resolution variant, a split-section pair, and opaque content between typed siblings can be built;
  - no PoE1-only field exists outside `GameSpecific::Poe1`;
  - the wasm32 build passes, with no new dependencies.
- **Depends on:** nothing.

### 2. Lift the item import pipeline into `loom-parser` (XL, split into four PRs)
Each sub-PR must keep Spike B's results. The maintainer runs `resolve-corpus` on Spike B's inputs and compares these `summary.json` fields, which must be equal: `non_unique` and `unique` (`LineCounts`: `lines`, `outcomes`, `by_section`, `with_shadowed_matches`); `rate` (`denominator`, `resolved`, `excluded`, `distinct*`, `stat_resolved`); `resolved_kinds`; the adapted counts; `ambiguous`; `categories`; `mod_backed`; `items`, `items_by_rarity`, `items_by_scope`, `line_kinds`. `tool` may differ. So may the input hashes once step 6 reads a bundle.
- **2a. Item input (M).**
  - Move `pob_item` to `loom-serialization::legacy::pob_item`, with its tests unchanged.
  - Add `legacy::pob_item::to_domain`, which maps a read item onto the domain item, PoB-only rules included.
  - Switch the parity-cli pipeline to the domain item.
  - Depends on 1.
- **2b. Scope and bases (M).**
  - Add the `ItemData` trait, starting with bases and `item_rules`.
  - Move base lookup and scope to `loom-parser::import`.
  - parity-cli implements `ItemData` over the extract. Its `item_rules` come from a `poe1` constant module in parity-cli until step 3 puts them in the bundle.
- **2c. Mods filter and item tags (M).** Move `mod_filter` and `item_tags`. Generation types, influences, and essence columns are read through `ItemData::item_rules`; no constant stays in shared code.
- **2d. Compositions, grafts, properties, tolerances (M).**
  - Move `grafts`, `properties`, case-fold, and split-section.
  - `COMPOSITIONS` (`ClientStrings` wrapper keys) becomes per-game data read through `ItemData`. The PoE1 list moves with it.
  - The categories and the PoB legacy-text evidence stay in the tool.
- **Accept (each):**
  - the field list above is equal;
  - moved unit tests pass unchanged;
  - no new dependency in `loom-parser`;
  - the wasm32 build passes;
  - the PoE1-identifier scan (gate 3c) covers the moved code.

### 3. Canonical game-data model and bundle v1 (L, may split into 3a and 3b)
- **3a, core (`loom-data`):**
  - canonical types for what import and validation read now: stats; mods (with families and types); base items; item classes; tags; `ClientStrings` entries the pipeline names; stat-description files; lookup tables; `item_rules` per game; the `poe1` section (essences, alternate quality types, grafts and graft skill levels, and the skill gems and granted effects the grafts name);
  - `bundle.json` with reserved `signatures`;
  - `BundleSource` with `list()` and `read()`;
  - the loader, with typed errors;
  - synthetic PoE1 and PoE2 bundles in `crates/loom-data/tests/fixtures/`.

  Passive, character, and full skill tables are not read in Phase 2, so they wait for Phase 3. Their IDs in a Build are kept but not validated in Phase 2.
- **3b, tool:** `loom-extractor bundle <extract-dir> --out <bundle-dir>`. It maps dat-schema rows to canonical types (decision 8) and writes `item_rules` from the vendored schema's enums.
- **Accept:**
  - the synthetic bundles load in CI;
  - the maintainer runs `bundle` twice on each of the PoE1 and PoE2 extracts, with byte-identical output;
  - the PoE2 bundle loads into the same types and has no `poe1` section;
  - scrubbed summaries (row counts per type, hashes) are committed;
  - `cargo deny` passes with `sha2`.
- **Depends on:** nothing (it can run in parallel with step 2).

### 4. Bundle validator (M)
`loom-data::validate` runs five kinds of check:
- **Structural:** the manifest format is known; every listed file exists, with matching SHA-256 and size; there is no unlisted file (`list()`).
- **Schema:** each file parses into its canonical type.
- **Referential:** every reference resolves, for example mod stats to stats, graft skills to gems, base items to item classes.
- **Domain:** the game tag matches the sections present; a PoE2 bundle has no `poe1` section; IDs are unique; `item_rules` exist for the game.
- **Smoke:** stat descriptions load, and a sample renders.

`loom-parity-cli bundle-check <dir>` prints a JSON report and exits 0 when the bundle is valid, 1 when it is not.
- **Accept:**
  - a mutation set in CI gives exactly its expected code for each case: missing file, changed byte, extra file, dangling reference, duplicate ID, wrong game tag, `poe1` section in a PoE2 bundle, unknown format version;
  - the synthetic valid bundles pass;
  - the maintainer runs it on the real PoE1 and PoE2 bundles with 0 errors.
- **Depends on:** 3.

### 5. PoB XML import and semantic export, structure only (L, may split into 5a and 5b)
`loom-serialization::legacy::pob_xml` reads XML into a Build and writes a Build back as XML (decisions 1, 2, 3, 6). This step also defines the round-trip comparison (decision 12): `xml_equal(a, b)` over parsed trees, plus the closed list of extra normalizations, each with a reason and a test. Step 8 reuses it.
- **Accept:**
  - for each public fixture: `import(export(import(x))) == import(x)`, and `xml_equal(export(import(x)), x)`;
  - the unknown-content mutations survive: an unknown attribute, child, and section; content between typed siblings; several `<Item>` text children; comments;
  - malformed XML gives `IMPORT_*` codes, never a panic;
  - the wasm32 build passes.
- **Depends on:** 1, 2a.
- **Possible split:** 5a covers Build, Tree, Skills, Config, and the opaque sections; 5b covers Items, item sets, and slots.

### 6. Resolve a Build's item lines (M)
`loom_parser::import::resolve_build` runs step 2's pipeline over every item of a Build, with an `ItemData` from a loaded bundle. It maps the parser's outcomes onto the domain line (stat IDs, typed `kind`, or unresolved with a reason; plus `adapted`; plus `continues` for split pairs) and sets completeness. This step adds the bundle's implementation of `ItemData`.
- **Accept:**
  - CI tests over the synthetic PoE1 and PoE2 bundles cover each outcome and each `kind`;
  - in a corpus run through the bundle (not the extract), run by the maintainer, step 2's field list equals Spike B's;
  - source lines in equal source lines out for every item.
- **Depends on:** 1, 2, 3, 5.

### 7. Successor JSON DTOs v1 and generated schemas (M)
- `loom-serialization::dto::v1`, with serde;
- deterministic conversion both ways: stable order everywhere, and no map whose iteration order can change;
- `tools/schema-gen`, with the JSON Schema and TypeScript committed and a CI freshness check.
- A CI job compares the SHA-256 of the JSON produced on the Windows runner with the one from the Linux runner, as Spike C's `oracle cross-platform` job does.
- **Accept:**
  - `Build -> DTO -> JSON -> DTO -> Build` returns an equal Build for synthetic PoE1 and PoE2 builds and for every public fixture import;
  - the JSON bytes are identical across two runs and across the two OSes;
  - a test validates the produced JSON against the schema;
  - `cargo deny` passes.
- **Depends on:** 1, 5.

### 8. Round-trip harness and CI gate job (M)
`loom-parity-cli build-round-trip <builds...> [--bundle <dir>] --out <dir> [--anonymous]` runs this sequence for each build:
1. Import the XML (Build A).
2. Resolve A when a bundle is given.
3. Write A to JSON and read it back (Build B).
4. Export B to XML and re-import it (Build C).
5. Resolve C with the same bundle.

Then it checks:
- **(a)** `xml_equal(export(B), original)`;
- **(b)** A, B, and C are equal, resolutions included (C was re-resolved, so resolution is compared too);
- **(c)** the JSON bytes are equal across two runs;
- **(d)** source lines in equal source lines out for every item.

It writes `report.json` and `summary.json`. The summary is scrubbed with the same code `resolve-corpus` uses; with `--anonymous`, builds are named by SHA-256. It also runs the unknown-content mutation set from step 5 over every section. A CI job runs it on the public fixtures (no bundle) and on the synthetic bundles.
- **Accept:**
  - the CI job is green;
  - the exit codes are documented;
  - the maintainer runs it on the 21-build corpus with the PoE1 bundle and commits the scrubbed summary.
- **Depends on:** 5, 6, 7.

### 9. Migrations and file persistence (M)
Implements decision 14 (migrations) and decision 13 (persistence).
- **Accept:**
  - a test-only v0-to-v1 migration produces its report;
  - a newer document is refused (`MIGRATE_TOO_NEW`);
  - a damaged file gives a `PERSIST_*` code and nothing is written;
  - save never targets the import source;
  - the atomic write holds when the write fails after the temporary file is written (tested with an injected writer).
- **Depends on:** 7.

### 10. Share codes (S)
Import and export PoB share codes through the XML adapter (decision 16).
- **Accept:** two codes each import to the same Build as their XML: one made by our exporter from a public fixture, and one made by PoB v2.67.2 (through the oracle runtime, recorded with its source XML).
- **Depends on:** 5.

### 11. Report and gate verdict (S-M)
- Write `docs/phase-2/phase-2-report.md`: what was built, the inputs, the gate verdict per item with its commands, the scrubbed summaries, and known limitations.
- Update the handoff, CLAUDE.md, and the ACTION-PLAN.
- Move the task folder to `dev/completed/`.
- **Depends on:** 4, 8, 9, 10.

## Gate (objective, checkable)
All of the following must hold on the recorded inputs. The summaries record the PoE1 and PoE2 Steam builds, which are the Spike A and B builds or newer.
1. **Semantic round-trip.** `build-round-trip` exits 0 on:
   - (a) every public fixture, in CI;
   - (b) the maintainer's 21 builds with the PoE1 bundle, with the scrubbed summary committed.

   Checks (a) to (d) of step 8 hold for every build. CI also compares the JSON bytes from Windows and Linux. PoB share codes import to the same Build as their XML (step 10).
2. **Unknown content preserved.** Three checks:
   - the mutation set passes in CI: every injected attribute, element, section, in-between content, extra item text, and comment survives XML to JSON to XML;
   - on the corpus, source lines in equal source lines out for every item, and every unresolved line is quarantined with a reason;
   - the corpus resolution through the bundle gives the Spike B field list from step 2.
3. **PoE2 bundles validate, with no PoE1 assumptions in shared crates.** Four checks:
   - (a) `bundle-check` exits 0 on the maintainer's PoE2 bundle, which loads into the same `loom-data` types with no `poe1` section;
   - (b) in CI, a synthetic PoE2 bundle with no PoE1 tables loads, validates, and drives `resolve_build` over a synthetic PoE2 Build, and that Build round-trips through DTO and JSON;
   - (c) a CI test scans the shared core crates' sources outside `poe1` modules and `legacy` and fails on PoE1-only identifiers, with the list kept in the test: `Bandit`, `Pantheon`, PoE1 table names (`Essences`, `AlternateQualityTypes`, `Brequel*`, `IndexableNonActiveSupportGems`, `IndexableSkillGems`), and the `ModDomain` and generation-type numbers as named constants;
   - (d) the game-specific table list agrees with the extractor's lists.
4. **Invalid bundles rejected.** Every case of the validator mutation set returns its expected code in CI.
5. **Determinism.** Two `bundle` runs give byte-identical bundles (maintainer). DTO and JSON output are byte-identical across runs and across platforms (CI).
6. **Hygiene.** Every core crate still builds for wasm32. The schema freshness check and `cargo deny` are green.

## Fallback
- **Round-trip fails for some content:** that content becomes opaque (kept as read) instead of typed. The report records the narrowing and the reason, and the gate is measured again. Phase 3 types the content when it needs it.
- **PoE2 tables do not fit the shared canonical types:** split the type into a shared core and game-tagged sections (decision 5). If the boundary itself has to change, propose an amendment to ADR-001 or ADR-005 and wait for the maintainer.
- **Spike B fields change after a lift PR:** stop. The lift must not change behavior, so find the difference before the next sub-PR.
- **A dependency fails the license or wasm check:** use the alternative named in its PR. The XML writer is already our own. If `miniz_oxide` is refused, share codes need an ADR-005 discussion with the maintainer, because the ADR makes them first-class.

## Out of scope (named so the gap is visible)
- Calculations and stat-to-effect mappings (Phase 3).
- Validating passive node, class, and gem IDs against data (Phase 3, with those tables).
- Command history, undo, and the single-writer build task (Phase 4, `loom-app`).
- The SQLite index (Phase 4, Q3).
- Bundle compression, the distribution container, real signatures, and the update channel (Phase 5, Q2).
- PoE2 build import (`.build`, backlog per ADR-005).
- Trade and clipboard item text (later import adapters over the same pipeline).
- Any UI.

## Risks and mitigations
| Risk | Mitigation |
|---|---|
| The typed model grows to cover everything PoB writes | Decision 1: only what Phase 3 needs is typed; everything else is opaque and preserved; each PR names what it types |
| The lift changes resolution results | Four lift PRs, each proving the step 2 field list unchanged before the next |
| PoE1 or PoB rules slip into shared code during the lift | Decision 5: rules are bundle data or `legacy`; gate 3c scans shared code; gate 3b drives the pipeline with PoE2 data in CI |
| PoB XML quirks: several text children per `<Item>`, legacy `ModRange`, CRLF, entities, variant lines, comments | Decisions 1 to 3 and 12; a mutation or unit test for each quirk |
| Formatting of typed values breaks the round-trip | Decision 2: source strings are kept and written back unchanged |
| Normalization hides real differences | Parsed-tree comparison plus a closed list of extra rules, each with a test and a reason |
| Corpus privacy (ADR-007) | Only scrubbed summaries leave `local/`; `--anonymous`; the same scrubbing code as `resolve-corpus` |
| Schema generation churn breaks the UI build | Generated files are committed and checked; TypeScript is generated from the JSON Schema, so there is one source |
| The phase is too large for one plan | Fourteen or more PRs, each one session; step 2 is pre-split, and the splits for steps 3 and 5 are named |
| CI has no game data, so the real gate is maintainer-run | The Spike A and B model: maintainer runs, scrubbed summaries with input hashes committed; CI covers synthetic PoE1 and PoE2 data and the public fixtures (Q1 could move real data into CI) |

## Planned PR breakdown
Each PR is one session, small and single-purpose, and fills in the evidence sections of the PR template. Evidence marked "maintainer-run" is pasted as scrubbed summaries.

| PR | Content | Evidence |
|---|---|---|
| P2-0 | This plan (docs only) | None for tests (docs only); plan-reviewer summary in the context file |
| P2-1 | Step 1, domain model | Unit tests; wasm32 build; coverage: domain types |
| P2-2a | Step 2a, item input, `pob_item` move | Moved tests; Spike B fields equal (maintainer-run) |
| P2-2b | Step 2b, scope and bases | Same, plus the PoE1-identifier scan starts |
| P2-2c | Step 2c, Mods filter and item tags | Same |
| P2-2d | Step 2d, compositions, grafts, properties, tolerances | Same |
| P2-3 | Step 3 (or 3a and 3b), data model, bundle v1, converter | Loader tests on synthetic bundles; two-run determinism of `bundle` on PoE1 and PoE2 (maintainer-run, summaries); `cargo deny` |
| P2-4 | Step 4, validator | Mutation set in CI with codes; `bundle-check` on the real PoE1 and PoE2 bundles (maintainer-run) |
| P2-5 | Step 5 (or 5a and 5b), XML import and export | Round-trip and mutation tests on public fixtures; malformed-input tests |
| P2-6 | Step 6 (split: 6a bundle `ItemData`, 6b `resolve_build`) | 6a: `bundle-equal` against the extract readers and corpus fields through the bundle equal Spike B (maintainer-run); 6b: outcome tests on synthetic PoE1 and PoE2 bundles |
| P2-7 | Step 7, DTO v1 and schemas | DTO round-trip tests; cross-OS byte comparison job; schema freshness job; `cargo deny` |
| P2-8 | Step 8, `build-round-trip` and CI job | CI job on fixtures, synthetic bundles, and the mutation set; corpus summary (maintainer-run) |
| P2-9 | Step 9, migrations and persistence | Migration and persistence tests |
| P2-10 | Step 10, share codes | Share-code tests |
| P2-11 | Step 11, report and gate | Gate commands and their outputs |

Steps 2 and 3 do not depend on each other, but they are done one per session in table order. P2-3 may come before P2-2b if the maintainer prefers.

**Parity and divergence.** Phase 2 computes nothing, so there is no oracle diff. Any difference from PoB's own reading of a build (for example an item line PoB parses and we quarantine) is listed in the report, and those that need it go to the divergence register (ADR-006).
