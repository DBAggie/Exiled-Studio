# ADR-007: Parity Corpus and Coverage

- Status: Accepted (revised 2026-11-08; corpus count corrected to 21 on 2026-11-10 at the maintainer's request, after the maintainer confirmed 21 builds on 2026-11-09)
- Date: 2026-02-01

## Decision
Use a layered corpus of real builds, synthetic edge cases, known-issue reproducers, and stress fixtures. Coverage is pairwise/orthogonal rather than exhaustive enumeration.

The initial corpus is seeded from the maintainer's own PoE1 builds (21 PoB XML files), scrubbed of account names and machine-specific metadata, plus synthetic fixtures. Legacy PoB ships only one test build, so this project builds its own corpus. Public community builds are added only where their terms permit or the author consents.

Small fixtures live in Git; large fixtures use LFS or versioned artifacts with manifests and hashes. Mechanic and behavior coverage are the primary parity metrics; code coverage is supporting evidence. Every discrepancy adds a minimized regression fixture and coverage metadata.
