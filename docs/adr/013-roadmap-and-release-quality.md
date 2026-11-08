# ADR-013: Roadmap, Release Quality, and Performance

- Status: Accepted (revised 2026-11-08)
- Date: 2026-02-01

## Decision
Use a weighted roadmap with hard gates. The sequence is defined in `docs/plan/ACTION-PLAN.md`: bootstrap, three ordered feasibility spikes (extraction, stat resolution, thin oracle), canonical data/model, narrow calc slice, application slice, packaging/security, then progressive expansion and PoE2 calculations.

Use a layered test pyramid, layered accessibility validation, layered security assurance, and layered performance validation. Performance uses hardware-tier absolute budgets plus relative regression limits. Correctness is invariant across hardware tiers.

## First usable milestone (revised)
A **narrow archetype slice**, not "any imported build":
- Archetypes: one self-cast spell and one non-triggered attack, no minions, with core defences.
- Any build can be imported and displayed. Calculations outside the supported archetypes show an explicit partial/unsupported state.
- Includes: import (PoB XML), dashboard, structured calculation breakdowns, item edit, async recalculation, save successor JSON, export semantic legacy XML, and the local MCP addon (ADR-017).

## Consequences
- Popularity alone does not determine scope.
- Every accepted bug/feature gets tests or acceptance criteria.
- The current application remains independently governed; no deprecation policy is imposed on it.
