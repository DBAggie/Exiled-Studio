# ADR-006: Parity Verification and Legacy Oracle

- Status: Accepted (revised 2026-11-08)
- Date: 2026-02-01

## Decision
Use a combined verification harness: differential testing against a pinned released Path of Building, a layered golden corpus, property/invariant tests, and permanent regression fixtures.

The oracle is a **thin harness**. A Rust CLI drives the legacy `HeadlessWrapper.lua` on pinned LuaJIT and emits normalized JSON of `mainOutput` and selected breakdowns. Legacy source is acquired from the public repository at a pinned tag and is not copied into the successor repository. The legacy project already runs headless in its own CI, so launch feasibility is not treated as a major risk. The open risk is normalizing output that is not a stable contract.

Oracle inputs, source tag/commit, runtime, data version, platform, and corpus revision are recorded on every run.

## Authority and divergence register
Per ADR-001, the game is the truth and the oracle is for regression. The **divergence register** is a versioned file listing each accepted difference: mechanic, fixture, oracle value, successor value, evidence, and the test that pins it. Unregistered differences fail the gate.

## Comparison rules
Layered comparison is required: exact discrete/build semantics, field-specific numerical tolerances, semantic provenance comparison, and workflow-level acceptance. No broad undocumented "close enough" rule is allowed.
