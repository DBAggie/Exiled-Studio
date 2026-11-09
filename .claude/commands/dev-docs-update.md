---
description: Update dev/active task docs before a context reset or at the end of a session
argument-hint: Optional - specific task or context to focus on
---

<!-- Adapted from diet103/claude-code-infrastructure-showcase (MIT, Copyright (c) 2025 Claude Code Infrastructure Contributors) for the loom project. -->

Update the task documentation so a new session can continue without loss.

For each task in `dev/active/`:
- `<task>-context.md`: current state, decisions made this session and why, files changed, blockers, exact next steps, and commands to verify the work. Update `Last Updated`.
- `<task>-tasks.md`: mark completed items, add newly discovered tasks, note in-progress status.

Also record:
- Any new divergence from the PoB oracle with its evidence, and whether it is in the divergence register yet.
- Any decision that conflicts with or extends an ADR. Flag it for the maintainer; do not edit ADRs without approval.
- Unfinished work: the exact file and goal being edited, uncommitted changes, and temporary workarounds.

Then update the repository-level handoff (each PR is done in a fresh session; see `AGENTS.md`, "One PR per session"):
- `docs/handoff/SUCCESSOR-HANDOFF.md`: rewrite **Current state** and **Next action**.
- `CLAUDE.md` Status and `docs/plan/ACTION-PLAN.md` Next action, if the phase, gate status, or next step changed.

Timing (see `AGENTS.md`, "When to update"): run this before opening the PR, and again right after `gh pr create`, to write the new PR number into the handoff Current state, the task file's "Open PR" item, and every other doc that names the PR. Push that as a follow-up commit to the PR branch before any other work.

Move a task folder to `dev/completed/` only when its gate has passed and the maintainer agrees.

Focus on what would be hard to rediscover from the code alone. Additional context: $ARGUMENTS
