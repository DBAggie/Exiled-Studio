---
description: Create a persistent task plan (plan, context, tasks) under dev/active that survives context resets
argument-hint: The task to plan, e.g. "Spike A - Bundles2 extraction"
---

<!-- Adapted from diet103/claude-code-infrastructure-showcase (MIT, Copyright (c) 2025 Claude Code Infrastructure Contributors) for the loom project. -->

Create an actionable, persistent plan for: $ARGUMENTS

## Before planning
- Read `docs/plan/ACTION-PLAN.md` and every ADR the task touches. The plan must fit them; if it can't, say which ADR needs an amendment and stop for the maintainer's decision.
- Examine the relevant existing files to understand the current state.

## Produce
Create `dev/active/<task-name>/` with three files, each starting with `Last Updated: YYYY-MM-DD`:

1. `<task-name>-plan.md`
   - Summary and which Action Plan phase/spike this belongs to
   - Current state and target state
   - Steps, each with acceptance criteria, dependencies, and effort (S/M/L/XL)
   - Gate: objective pass/fail criteria that CI or a script can check
   - Fallback if the gate fails
   - Risks and mitigations
   - Planned PR breakdown: small, single-purpose PRs, each listing the evidence it will carry (tests, parity/divergence diff, coverage touched)
2. `<task-name>-context.md`
   - Key files, ADRs, external references, decisions and their reasons, open questions
3. `<task-name>-tasks.md`
   - Checklist of tasks in order

Then recommend running the `plan-reviewer` agent on the plan before implementation starts.
