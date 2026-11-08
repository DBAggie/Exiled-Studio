<!--
One purpose per PR (ADR-012). CI fails if any evidence section below is empty.
Write "None: <reason>" when a section does not apply, e.g. "None: docs only".
-->

## Summary
<!-- What changes and why. Link the issue, ADR, or dev/active task. -->

## Tests
<!-- Commands run and results; new or changed tests. -->

## Parity / divergence
<!-- Oracle comparison diff, new divergence-register entries with evidence (ADR-001/006). -->

## Coverage touched
<!-- Mechanics, data tables, or contracts this PR affects (ADR-007). -->

## Handoff
<!-- Each PR is done in a fresh session. List the docs updated so the next session can resume
     (handoff Current state + Next action on every PR; see AGENTS.md "One PR per session"). -->

## Checklist
- [ ] Single purpose and small
- [ ] No ADR contradicted (or an amendment is linked)
- [ ] Core crates stay wasm32-compatible (ADR-016)
- [ ] No GGG assets added (ADR-000); no telemetry (ADR-011)
