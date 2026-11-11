# Phase 2, step 8: the round-trip gate (`build-round-trip`)

Plan step 8 and gates 1, 2, and 3b (`dev/completed/phase-2-canonical-model/phase-2-canonical-model-plan.md`). The command and its checks are documented in `tools/parity-cli/src/build_round_trip.rs` (module docs) and in `loom-parity-cli`'s usage text.

## What runs
For each build, `loom-parity-cli build-round-trip`:
1. reads the input (Build A): a PoB build (`*.xml`) or a v1 build document (`*.json`);
2. resolves A when `--bundle` is given (the bundle is validated first);
3. writes A as the v1 JSON document and reads it back (Build B);
4. exports B as PoB XML, imports it again, and resolves it with the same bundle (Build C). PoB's format is PoE1's, so a PoE2 build has no C.

Then it checks:
- **(a)** `xml_equal(export(B), input)` (PoB input);
- **(b)** B and C equal A, resolutions included;
- **(c)** steps 1 to 3, run twice, give the same JSON bytes;
- **(d)** every typed item's source lines equal the lines of its text in the input, in A after resolution, in B, and in C.

PoB input also runs the **unknown-content mutation set** (`build_round_trip/mutate.rs`). On the first element of every distinct element path, it applies each kind that fits: an unknown attribute; an unknown element as the first and as the last child (on the root, an unknown section); a comment before the element; a processing instruction after it; an unknown element right after it (between typed siblings); and, on `Items/Item`, a second, earlier item text. Each mutated copy must import, read back from JSON to an equal build, and export to a document `xml_equal` to the copy. The values need escaping (`< & > "`, tab, newline).

**Exit codes:** 0 when every build passes; 1 when a build fails, or when no build was given; 2 on a usage error; 3 when the run cannot start (an input or the bundle does not read or validate, or `--out` exists).

## CI (`build round-trip` job on Windows and Linux, required through `ci-ok`)
- the 24 public fixtures (`tools/oracle/fixtures`, `tools/oracle/fixtures/community`), with no bundle. `malformed.xml` must not import (`--expect-import-error`), so 25 inputs;
- the synthetic PoE1 PoB build (`tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml`) with `bundle-poe1`, and the synthetic PoE2 build document (`synthetic-poe2.json`, made from the PoE1 build's items) with `bundle-poe2`. This is gate 3b: a PoE2 bundle with no PoE1 tables drives `resolve_build` over a PoE2 build, and that build round-trips through the JSON document. The job also requires resolved lines and no pending ones;
- the PoE1-identifier scan (gate 3c, `cargo test -p loom-parser --test no_poe1_identifiers`, Linux);
- `dto cross-platform` diffs every document's SHA-256 from these runs between Windows and Linux.

In-session, 2026-11-10, at `d1aa9b7`, on Windows and on WSL Ubuntu 24.04 (the job's commands, with the `jq` conditions checked in Python because WSL had no `jq`):
- public fixtures: 25 inputs, 25 pass: (a) to (d) pass for 24 and `malformed.xml` gives `IMPORT_XML` as expected. 4,492 mutations, all survived;
- synthetic PoE1: (a) to (d) pass. Modifier lines: 9 resolved, 2 adapted, 2 continuations, 6 unresolved, 1 excluded. Every outcome of the default scope comes through the XML adapter;
- synthetic PoE2: (b) to (d) pass, (a) is not applicable. 7 resolved, 2 adapted, 2 continuations, 8 unresolved, 1 excluded. The PoE2 data has no quality types and no granted skills;
- the documents' SHA-256 are equal on Windows and Linux for all 27 builds.

## Corpus (gate 1b, maintainer-run)
The maintainer's 21 builds, run with the PoE1 bundle that P2-6 used (`bundle.json` SHA-256 `6c628cd6…`, Steam build 26530219), with `--anonymous`:

```
cargo run --release -p loom-parity-cli -- build-round-trip "<builds-dir>" --bundle <bundle-dir> --anonymous --out <dir>
```

- Run twice at `d1aa9b7` from a clean tree, exit 0 both times. `summary.json` and `report.json` are byte-identical between the runs.
- **21 of 21 builds pass (a) to (d).** 1,055 items and 18,842 source lines. Modifier lines: 4,121 resolved, 23 adapted, 49 continuations, 397 unresolved, 1,471 excluded. That makes 6,061 lines, the count P2-6b compared with the harness. Completeness: 5 complete, 16 partial. Nothing kept whole.
- Mutations: 3,990 copies, all survived.
- The scrubbed summary is committed as `docs/phase-2/build-round-trip-poe1-26530219.json` (builds named by SHA-256; counts only). `report.json` lists no failures.
