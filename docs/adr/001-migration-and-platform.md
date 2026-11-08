# ADR-001: Migration Posture, Authority, and Platforms

- Status: Accepted (revised 2026-11-08)
- Date: 2026-02-01

## Decision
Build a full rewrite, starting with Path of Exile 1, on a core that is ready for PoE2 from the start. PoE2 game data goes through the same extraction and data pipeline from Phase 2 to validate the game-module boundary. PoE2 calculations begin after the PoE1 application-slice gate (Phase 4), not after a PoE1 beta.

Platforms: Windows and Linux are release gates. macOS is built in CI on a best-effort basis and is promoted to a release gate once it can be tested.

## Calculation authority (revised)
**The game is the source of truth.** The released Path of Building application, pinned to a specific release (initially v2.67.2), is the **regression oracle**, not the authority.
- Every difference from the oracle must be explained.
- A difference is accepted as intentional only with evidence (game data, a documented in-game test, or official GGG documentation). It is recorded in the divergence register (ADR-006) with a test.
- Unexplained differences block release for the affected mechanic.
- Matching PoB is the starting gate for each mechanic. It is not the permanent goal, and the successor does not track every upstream PoB change.

## Consequences
- No rolling, unfinishable target.
- PoE1 assumptions must not leak into shared crates. PoE2 data in the pipeline from Phase 2 enforces this.
- Divergence evidence is a first-class project artifact.
