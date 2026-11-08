# ADR-004: Application API, State, and Concurrency

- Status: Accepted
- Date: 2026-02-01

## Decision
Use a command/query/event API without full event sourcing. Commands express intent and create undoable history entries; queries return typed projections; events report committed changes, jobs, progress, and errors.

One application task owns the mutable Build aggregate and processes commands sequentially. Successful commands produce versioned immutable snapshots. React uses Redux Toolkit for UI/read projections and targeted state machines for multi-step workflows. State synchronization uses versioned events plus recovery queries and full snapshots.

## Consequences
- Rust remains authoritative.
- React never patches canonical domain state directly.
- Missing event versions trigger projection refresh.
- Background jobs consume snapshots and cannot mutate canonical state directly.
