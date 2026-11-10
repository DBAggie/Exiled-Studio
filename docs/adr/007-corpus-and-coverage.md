# ADR-007: Parity Corpus and Coverage

- Status: Accepted (revised 2026-11-08; corpus count corrected to 21 on 2026-11-10 at the maintainer's request, after the maintainer confirmed 21 builds on 2026-11-09; Amendment 1, 2026-11-10: credited community builds with removal on request)
- Date: 2026-02-01

## Decision
Use a layered corpus of real builds, synthetic edge cases, known-issue reproducers, and stress fixtures. Coverage is pairwise/orthogonal rather than exhaustive enumeration.

The initial corpus is seeded from the maintainer's own PoE1 builds (21 PoB XML files), scrubbed of account names and machine-specific metadata, plus synthetic fixtures. Legacy PoB ships only one test build, so this project builds its own corpus. Public community builds are added only where their terms permit or the author consents.

Small fixtures live in Git; large fixtures use LFS or versioned artifacts with manifests and hashes. Mechanic and behavior coverage are the primary parity metrics; code coverage is supporting evidence. Every discrepancy adds a minimized regression fixture and coverage metadata.

## Amendment 1 (2026-11-10): Credited community builds with removal on request
Chosen by the maintainer on 2026-11-10 (Spike C, PR C3b+C4), after the terms below were found; approved by merging the PR that adds it. Evidence: `tools/oracle/fixtures/community/SOURCES.md`.

### Context
The oracle needs representative current builds; the maintainer's 21 builds stay local, and PoB ships only 5 builds from 3.13. The maintainer chose community staples from Maxroll (build guides) and pohx.net (Pohx's Righteous Fire builds) and wants them in the repository, so the community can see the test set and say when a build is not representative. The original text allows community builds "only where their terms permit or the author consents". Maxroll's guides fall under the Ziff Davis terms of use, which allow personal, non-commercial use and otherwise forbid reproducing or distributing the content; pohx.net states no terms. No site with permissive terms has current community staples: Mobalytics refuses automated access, poe.ninja's builds are players' characters with no stated license, and the MIT-licensed build repositories on GitHub hold personal builds from older patches.

### Decision
- Public community builds may be committed as **test fixtures** without a permissive license or the author's prior consent, when all of these hold:
  - each build is credited to its creator and source (URL, author, retrieval date, patch) in `tools/oracle/fixtures/community/SOURCES.md`, with what that source's terms say;
  - `SOURCES.md` opens with the removal-on-request note, and the repository `README.md` points to it; a creator's request removes the build, its fixture-manifest entry, and every register entry or test that names it, without discussion;
  - the build is used only to test the oracle and, later, the successor's calculations; it is not shown, sold, or distributed as content in the application;
  - account identifiers (`lastAccountHash`, `lastCharacterHash`) are removed; nothing else in the build is changed, and both the downloaded and the committed SHA-256 are recorded.
- Where a source's terms forbid reproduction (Maxroll at the time of writing), the maintainer accepts that risk for this test use; `SOURCES.md` says so next to the credit. Asking the creators for consent stays the preferred path and is recorded there when it happens.
- Builds whose terms permit it, or whose author consents, need none of this beyond the credit.

### Consequences
- The test set includes current, representative builds that anyone can inspect, at the cost of a known conflict with one site's terms, limited by credit, removal on request, and test-only use.
- A removal request can drop a fixture that tests depend on; tests must name community fixtures only through the fixture manifest so a removal is a data change.
- The maintainer's own corpus rule is unchanged: it stays local, scrubbed summaries only.
